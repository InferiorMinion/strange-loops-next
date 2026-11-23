"""Compare frozen legacy exports with the readings of their shadow stores."""

import argparse
from collections import Counter, defaultdict
from contextlib import ExitStack
from dataclasses import dataclass, field
from enum import Enum, auto
import errno
import hashlib
import json
import math
import os
from pathlib import Path
import re
import struct
import subprocess
import sys
import tempfile

from old_id import valid_old_id


class Invalid(ValueError):
    """Messages are public. Keep source text and child diagnostics out of them."""


def require(condition, message):
    if not condition:
        raise Invalid(message)


def load_json(raw):
    def distinct(pairs):
        obj = dict(pairs)
        require(len(pairs) == len(obj), "duplicate JSON member")
        return obj

    def bad_number(_):
        raise Invalid("invalid JSON number")

    return json.loads(raw, object_pairs_hook=distinct, parse_constant=bad_number)


class State(Enum):
    ABSENT = auto()
    CONFLICT = auto()
    UNKNOWN = auto()


def value(v):
    """Hashable JSON equality, including nested arrays and objects and true != 1."""
    if isinstance(v, dict):
        return ("object", frozenset((k, value(x)) for k, x in v.items()))
    if isinstance(v, list):
        return ("array", tuple(map(value, v)))
    if isinstance(v, bool):
        return ("bool", v)
    if isinstance(v, float):
        require(math.isfinite(v), "nonfinite payload number")
    return v


@dataclass
class Thing:
    key: str | None
    fields: dict = field(default_factory=dict)
    heads: int = 1
    unheld: bool = False


@dataclass
class Kind:
    keyed: dict = field(default_factory=dict)
    roots: list = field(default_factory=list)


@dataclass
class Cut:
    kinds: dict
    reading: dict = field(default_factory=lambda: defaultdict(Kind))
    facts: Counter = field(default_factory=Counter)
    fields: dict = field(default_factory=lambda: defaultdict(set))
    history: dict = field(default_factory=lambda: defaultdict(list))


def decimal(s, low, high):
    n = int(s)
    require(str(n) == s and low <= n < high, "invalid export integer")
    return n


def read_cut(facts, map_file, unnamed, keep_history):
    mapping = load_json(map_file.read_text(encoding="utf-8"))
    require(mapping["format"] == "shadow-map-v1", "unsupported map format")
    require(set(mapping) == {"format", "source", "store", "vertex_sha256", "kinds"}
            and re.fullmatch("[0-9a-f]{64}", mapping["vertex_sha256"]), "invalid map")
    cut = Cut(mapping["kinds"])
    for kind, fold in cut.kinds.items():
        require(kind, "empty map kind")
        if set(fold) == {"collect"}:
            decimal(fold["collect"], 0, 2**64)
        else:
            require(set(fold) == {"by"} and isinstance(fold["by"], str) and fold["by"],
                    "invalid map fold")
    with facts.open("rb") as stream:
        def decode(line):
            require(line.endswith(b"\n") and not line.endswith(b"\r\n"), "invalid export framing")
            return load_json(line.decode("utf-8"))

        line = stream.readline()
        header = decode(line)
        require(header["format"] == "shadow-facts-v1" and header["source"] == mapping["source"]
                and header["vertex_sha256"] == mapping["vertex_sha256"], "map and export disagree")
        require(set(header) == {"format", "source", "vertex_sha256", "order", "ticks"},
                "invalid export header")
        decimal(header["ticks"], 0, 2**64)
        axes = {"rowid": 1, "arrival": 2}[header["order"]]
        digest = hashlib.sha256(line).digest()
        ids = set()
        previous = ()
        for number, line in enumerate(stream, 2):
            row = decode(line)
            if "count" in row:
                require(row == {"count": str(len(ids)), "digest": digest.hex()}
                        and not stream.read(1), "invalid export trailer")
                return cut
            digest = hashlib.sha256(digest + line).digest()
            require(set(row) == {"coordinate", "id", "kind", "ts_bits", "observer", "origin",
                                 "payload", "signature"} and isinstance(row["coordinate"], list)
                    and all(isinstance(row[k], str) for k in
                            ("kind", "observer", "origin", "payload"))
                    and (row["signature"] is None or isinstance(row["signature"], str))
                    and re.fullmatch("[0-9a-f]{16}", row["ts_bits"]),
                    "invalid export row")
            old_id = row["id"]
            require(valid_old_id(old_id) and old_id not in ids,
                    f"shadow-source-invalid at line {number}: invalid or duplicate old id")
            coord = tuple(decimal(s, -2**63, 2**63) for s in row["coordinate"])
            kind = row["kind"]
            require(len(coord) == axes and coord > previous,
                    "invalid export coordinate or old id")
            previous = coord
            ids.add(old_id)
            if kind not in cut.kinds and not unnamed:
                continue
            payload = load_json(row["payload"])
            require(isinstance(payload, dict), "expected an object payload")
            fields = {k: value(v) for k, v in payload.items()}
            cut.fields[kind].update(fields)
            cut.facts[kind] += 1
            by = cut.kinds.get(kind, {}).get("by")
            if by not in payload:
                cut.reading[kind].roots.append(Thing(old_id, fields))
                continue
            key = payload[by]
            require(isinstance(key, str) and key, "invalid key value")
            cut.reading[kind].keyed.setdefault(key, Thing(key)).fields.update(fields)
            if keep_history:
                ts, = struct.unpack(">d", bytes.fromhex(row["ts_bits"]))
                require(math.isfinite(ts), "invalid occurrence time")
                cut.history[kind, key].append((ts, coord, fields))
    raise Invalid("missing export trailer")


class Loops:
    def __init__(self, binary, env):
        self.binary = binary
        self.env = env

    def run(self, store, *args, spec=None):
        proc = subprocess.run([self.binary, "-C", store, *args], env=self.env,
                              input=json.dumps(spec).encode() if spec else b"",
                              capture_output=True)
        require(proc.returncode == 0, f"loops {args[0]} failed")
        return load_json(proc.stdout.decode("utf-8"))

    def fold(self, stores):
        bindings = []
        sources = []
        for i, store in enumerate(stores):
            alias = f"source-{i}"
            bindings.extend(["--store", f"{alias}={store}"])
            sources.append({"store": alias})
        spec = {"vertex": "compare", "from": sources,
                "fold": {"items": {"latest-by": "of", "follow": "keys"}}}
        return self.run(stores[0], "fold", "-", "--values", *bindings, spec=spec)["items"]["things"]


def fold_thing(key, row):
    fields = {k: value(v) for k, v in row["values"].items()}
    for k, setters in row.get("setters", {}).items():
        fields[k] = State.CONFLICT if len(setters) > 1 else State.UNKNOWN
    return Thing(key, fields, len(row["heads"]), bool(row.get("unheld")))


def projection(cell):
    state = cell["state"]
    if state == "value":
        return value(cell["value"])
    return {"unset": State.ABSENT, "conflict": State.CONFLICT, "unknown": State.UNKNOWN}[state]


def read_shadow(loops, store, cut):
    records = {r["kind"]: r["records"] for r in loops.run(store, "ls")["kinds"]}
    reading = defaultdict(Kind)
    # A values fold also discovers fields present only in the shadow and the
    # payloads of roots, whose record ids cannot be derived from a legacy cut.
    for thing, row in loops.fold([store]).items():
        kind, key = thing.split(":", 1)
        if kind == "r":
            kind = row["heads"][0]["env"]["kind"]
            reading[kind].roots.append(fold_thing(thing, row))
        else:
            reading[kind].keyed[key] = fold_thing(key, row)

    keyed = {k for k, fold in cut.kinds.items()
             if "by" in fold and re.fullmatch("[a-z][a-z0-9-]*", k) and k != "r"}
    keyed.update(k for k, items in reading.items() if items.keyed)
    for kind in sorted(keyed):
        items = reading[kind].keyed
        names = cut.fields[kind].union(*(t.fields for t in items.values()))
        # The ls comma-list cannot express these names. The fold above can.
        names = sorted(n for n in names if n and "," not in n and "\0" not in n)
        args = ["--fields", ",".join(names)] if names else []
        try:
            rows = loops.run(store, "ls", kind + ":", *args)["things"]
        except OSError as error:
            if error.errno != errno.E2BIG or not names:
                raise
            # Some valid field lists exceed the OS exec argument limit.
            # Keep the complete field values from the bound fold in that case.
            names = []
            rows = loops.run(store, "ls", kind + ":")["things"]
        seen = set()
        for row in rows:
            prefix, key = row["thing"].split(":", 1)
            require(prefix == kind and key in items and key not in seen, "inconsistent shadow reads")
            seen.add(key)
            item = items[key]
            require(item.heads == row["heads"], "inconsistent shadow heads")
            for name in names:
                cell = projection(row["fields"][name])
                if cell == State.ABSENT:
                    item.fields.pop(name, None)
                else:
                    item.fields[name] = cell
        require(seen == items.keys(), "inconsistent shadow inventory")
    return records, reading


def root_pairs(old, new):
    """Root ids live in different namespaces. Compare payloads with multiplicity."""
    buckets = defaultdict(list)
    for item in new:
        if item.unheld:
            yield None, item
        else:
            buckets[frozenset(item.fields.items())].append(item)
    for item in old:
        bucket = buckets[frozenset(item.fields.items())]
        yield item, bucket.pop() if bucket else None
    for bucket in buckets.values():
        for item in bucket:
            yield None, item


def different_fields(left, right):
    return sorted(k for k in left.keys() | right.keys()
                  if left.get(k, State.ABSENT) != right.get(k, State.ABSENT))


@dataclass
class Examples:
    limit: int
    rows: list = field(default_factory=list)

    def add(self, source, comparison, kind, key, fields):
        if len(self.rows) < self.limit:
            self.rows.append({"source": source, "comparison": comparison, "kind": kind,
                              "key": key, "fields": sorted(fields)})


def thing_counts():
    return dict(old=0, new=0, equal=0, differing=0, only_old=0, only_new=0,
                fields_differing=0, fields_conflicting=0, multiple_heads=0)


def compare_kind(old, new, examples, source, comparison, kind):
    counts = thing_counts()
    counts.update(old=len(old.roots) + len(old.keyed), new=len(new.roots) + len(new.keyed))

    def compare(a, b):
        if b is not None:
            counts["multiple_heads"] += b.heads > 1
            counts["fields_conflicting"] += sum(v == State.CONFLICT for v in b.fields.values())
        if a is None or b is None:
            counts["only_new" if a is None else "only_old"] += 1
            item = b if a is None else a
            examples.add(source, comparison, kind, item.key, item.fields)
        else:
            fields = different_fields(a.fields, b.fields)
            if fields or b.unheld:
                counts["differing"] += 1
                counts["fields_differing"] += len(fields)
                examples.add(source, comparison, kind, a.key, fields)
            else:
                counts["equal"] += 1

    for key in sorted(old.keyed.keys() | new.keyed.keys()):
        compare(old.keyed.get(key), new.keyed.get(key))
    for a, b in root_pairs(old.roots, new.roots):
        compare(a, b)
    return counts


def disagrees(counts):
    return any(counts[k] for k in ("only_old", "only_new", "differing"))


def old_answer(path, cut):
    reading = defaultdict(Kind)
    unnamed = 0
    for row in load_json(path.read_text(encoding="utf-8"))["rows"]:
        kind, key, payload = row["kind"], row["key"], row["payload"]
        if kind not in cut.kinds:
            unnamed += 1
            continue
        require(key is None or type(key) in (str, int), "invalid old row key")
        # sl reserves metadata when materializing fold items. Preserve any
        # other underscore-prefixed field actually present in the cut.
        fields = {k: value(v) for k, v in payload.items() if k not in LEGACY_METADATA
                  and (not k.startswith("_") or k in cut.fields[kind])}
        by = cut.kinds[kind].get("by")
        if by and (by in payload or key in cut.reading[kind].keyed):
            require(isinstance(key, str) and key not in reading[kind].keyed, "invalid old row key")
            reading[kind].keyed[key] = Thing(key, fields)
        else:
            reading[kind].roots.append(Thing(None if key is None else str(key), fields))
    return reading, unnamed


LEGACY_METADATA = {"_ts", "_observer", "_origin", "_id", "_n", "_refs"}


def without_old_metadata(original):
    def clean(t):
        return Thing(t.key, {k: v for k, v in t.fields.items() if k not in LEGACY_METADATA})

    return Kind({k: clean(t) for k, t in original.keyed.items()}, list(map(clean, original.roots)))


def source_report(cut, records, shadow, old_json, examples, source):
    old_rows, excluded = old_answer(old_json, cut) if old_json else ({}, 0)
    kinds = sorted(cut.kinds.keys() | cut.facts.keys() | records.keys() | shadow.keys())
    total = Counter(thing_counts())
    mismatches = 0
    rows = []
    for kind in kinds:
        counts = compare_kind(cut.reading[kind], shadow[kind], examples, source, "shadow", kind)
        total.update(counts)
        row = {"facts": cut.facts[kind], "records": records.get(kind, 0), "things": counts}
        mismatches += row["facts"] != row["records"]
        if old_json and kind in cut.kinds:
            old = without_old_metadata(cut.reading[kind])
            row["old_json"] = compare_kind(old, old_rows[kind], examples, source, "old_json", kind)
        rows.append(row)
    summary = {"things": dict(total), "kinds": rows, "record_count_mismatches": mismatches}
    if old_json:
        summary["old_json_unnamed_rows"] = excluded
    return summary, bool(mismatches or disagrees(total) or
                         any(disagrees(r["old_json"]) for r in rows if "old_json" in r))


def union_report(cuts, stores, loops, examples):
    owners = Counter()
    for cut in cuts:
        owners.update(cut.history.keys())
    shared = [t for t, n in owners.items() if n > 1]
    union = loops.fold(stores) if shared else {}
    differing = multiple = 0
    for kind, key in sorted(shared):
        arrivals = [(ts, coord, i, fields) for i, cut in enumerate(cuts)
                    for ts, coord, fields in cut.history.get((kind, key), ())]
        old = {}
        for _, _, _, fields in sorted(arrivals, key=lambda entry: entry[:3]):
            old.update(fields)
        row = union.get(f"{kind}:{key}")
        actual = fold_thing(key, row) if row is not None else Thing(key)
        multiple += actual.heads > 1
        fields = different_fields(old, actual.fields)
        if row is None or fields or actual.unheld:
            differing += 1
            examples.add(None, "union", kind, key, fields)
    return {"shared": len(shared), "differing": differing, "multiple_heads": multiple}


class Arguments(argparse.ArgumentParser):
    def error(self, message):
        # argparse's default diagnostic can echo private operands.
        self.print_usage(sys.stderr)
        self.exit(2, "invalid comparison arguments (see --help)\n")


def main():
    parser = Arguments(description=__doc__)
    for name in ("facts", "map", "shadow"):
        parser.add_argument(name, type=Path)
    parser.add_argument("--loops", default="loops")
    parser.add_argument("--unnamed", action="store_true")
    parser.add_argument("--old-json", type=Path, help="old reader output for the first source")
    parser.add_argument("--also", nargs=3, type=Path, action="append", default=[],
                        metavar=("FACTS", "MAP", "SHADOW"))
    parser.add_argument("--show", type=int, default=0, metavar="N")
    args = parser.parse_args()
    if args.show < 0:
        parser.error("--show requires a nonnegative limit")
    inputs = [(args.facts, args.map, args.shadow), *args.also]
    examples = Examples(args.show)
    try:
        stores = [path.resolve() for _, _, path in inputs]
        require(len(set(stores)) == len(stores), "duplicate shadow directory")
        env = os.environ.copy()
        with ExitStack() as stack:
            if "LOOPS_CACHE_DIR" not in env:
                env["LOOPS_CACHE_DIR"] = stack.enter_context(tempfile.TemporaryDirectory())
            loops = Loops(args.loops, env)
            for store in stores:
                require((store / ".loops/segments").is_dir(), "missing shadow store")
                check = loops.run(store, "check")
                require(not check["altered"] and not check["refused"],
                        "shadow check failed")
            cuts = []
            reports = []
            different = False
            for i, (facts, mapping, _) in enumerate(inputs):
                cut = read_cut(facts, mapping, args.unnamed, len(inputs) > 1)
                cuts.append(cut)
                records, shadow = read_shadow(loops, stores[i], cut)
                report, differs = source_report(cut, records, shadow, args.old_json if i == 0 else None,
                                               examples, i + 1)
                reports.append(report)
                different |= differs
            result = {"sources": reports}
            if len(cuts) > 1:
                result["union"] = union_report(cuts, stores, loops, examples)
                different |= bool(result["union"]["differing"])
            if args.show:
                result["examples"] = examples.rows
    except Invalid as error:
        print(error, file=sys.stderr)
        return 1
    except (OSError, ValueError, KeyError, IndexError, TypeError, AttributeError, struct.error,
            RecursionError):
        print("unreadable or malformed comparison input or loops answer", file=sys.stderr)
        return 1
    print(json.dumps(result, separators=(",", ":")))
    return int(different)


if __name__ == "__main__":
    sys.exit(main())
