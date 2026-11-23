"""Comparison walks with invented SQLite snapshots and a fixture-backed loops."""

import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

from export import export


SCRIPT = Path(__file__).with_name("compare.py").resolve()
FAKE = '''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path

_, flag, root, verb, *args = sys.argv
assert flag == "-C"
root = Path(root)
data = json.loads((root / "reading.json").read_text())
cache = os.environ["LOOPS_CACHE_DIR"]
with (root / "calls").open("a") as log:
    print(json.dumps([verb, args, cache]), file=log)
if data.get("fail") == verb:
    sys.exit("private-child-error")
things = data["things"]
if verb == "check":
    out = {"altered": [], "refused": [], "unbound": 0, **data.get("check", {})}
elif verb == "ls" and not args:
    counts = {}
    for row in things.values():
        k = row["kind"]
        counts[k] = counts.get(k, 0) + row.get("records", 1)
    out = {"kinds": [{"kind": k, "records": n} for k,n in counts.items()]}
elif verb == "ls":
    names = args[2].split(",") if len(args) > 1 else []
    assert args[1:2] == (["--fields"] if names else [])
    rows = []
    for key, row in things.items():
        if not key.startswith(args[0]):
            continue
        fields = {}
        for name in names:
            if name in row.get("conflicts", []):
                fields[name] = {"state":"conflict"}
            elif name in row["values"]:
                fields[name] = {"state":"value", "value":row["values"][name]}
            else:
                fields[name] = {"state":"unset"}
        rows.append({"thing":key, "heads":row.get("heads", 1), "fields":fields})
    out = {"things":rows}
elif verb == "fold":
    spec = json.load(sys.stdin)
    assert spec["fold"] == {"items":{"latest-by":"of","follow":"keys"}}
    assert args[:2] == ["-", "--values"]
    assert args[2::2] == ["--store"] * len(spec["from"])
    bindings = dict(arg.split("=", 1) for arg in args[3::2])
    assert set(bindings) == {s["store"] for s in spec["from"]}
    assert all(Path(store, "reading.json").exists() for store in bindings.values())
    if len(bindings) > 1:
        things = data["union"]
    out = {"items":{"things":{
        key: {"values":row["values"],
              "heads":[{"env":{"kind":row["kind"]}}] * row.get("heads", 1),
              "setters":{k:[{},{}] for k in row.get("conflicts", [])}}
        for key,row in things.items()}}}
else:
    sys.exit(2)
print(json.dumps(out))
'''


def thing(kind, fields, **extra):
    return {"kind": kind, "values": fields, **extra}


def write_json(path, data):
    path.write_text(json.dumps(data), encoding="utf-8")


def source(tmp, name, facts, shadow):
    home = tmp / name
    home.mkdir()
    vertex = home / "source.vertex"
    vertex.write_text(f'name "{name}"\nstore "cut.sqlite"\nloops {{\n'
                      'thread { fold { items "by" "name" } }\n'
                      'change { fold { items "collect" 2 } }\n}\n', encoding="utf-8")
    db = sqlite3.connect(home / "cut.sqlite")
    try:
        db.execute("CREATE TABLE ticks(id TEXT)")
        db.execute("CREATE TABLE facts(id TEXT PRIMARY KEY, kind TEXT, ts REAL, observer TEXT,"
                   "origin TEXT, payload TEXT, signature TEXT, arrival_ordinal INT, arrival_seq INT)")
        for i, (kind, ts, coord, fields) in enumerate(facts):
            db.execute("INSERT INTO facts VALUES(?,?,?,'','',?,NULL,?,?)",
                       (f"{i:026}", kind, ts, json.dumps(fields), *coord))
        db.commit()
    finally:
        db.close()
    export(vertex, home / "export")
    store = home / "shadow"
    (store / ".loops/segments").mkdir(parents=True)
    write_json(store / "reading.json", {"things": shadow})
    return [home / "export" / (name + ".facts.jsonl"), home / "export" / (name + ".map.json"), store]


class CompareTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name)
        self.fake = self.tmp / "loops"
        self.fake.write_text(FAKE, encoding="utf-8")
        self.fake.chmod(0o700)
        self.kiln = {"name": "kiln", "status": "secret-open", "temp": None,
                     "_user": "secret-text", "odd,key": "secret-comma"}
        self.missing = {"label": "secret-root"}
        self.changes = {"nested": [True, {"heat": 1}], "comma,key": "secret-field"}
        self.things = {
            "thread:kiln": thing("thread", self.kiln, records=2),
            "r:missing": thing("thread", self.missing),
            "r:one": thing("change", self.changes),
            "r:two": thing("change", self.changes),
        }
        self.args = source(self.tmp, "workshop", [
            ("thread", 0.5, (2, 0), {"name": "kiln", "status": "secret-open"}),
            ("thread", 1.0, (1, 7), {**self.kiln, "status": "before"}),
            ("thread", 2.0, (2, 1), self.missing),
            ("change", 3.0, (3, 0), self.changes),
            ("change", 4.0, (4, 0), self.changes),
            ("moved", 5.0, (5, 0), {"other": 0}),
        ], self.things)

    def run_compare(self, *extra, code=0, env=None):
        child = dict(os.environ)
        child.pop("LOOPS_CACHE_DIR", None)
        child.update(env or {})
        out = subprocess.run([sys.executable, SCRIPT, *self.args, "--loops", self.fake, *extra],
                             capture_output=True, text=True, env=child)
        self.assertEqual(out.returncode, code, (out.stdout, out.stderr))
        return out

    def reading(self, **extra):
        write_json(self.args[2] / "reading.json", {"things": self.things, **extra})

    def test_equal_private_and_cache(self):
        out = self.run_compare()
        for private in ("workshop", "kiln", "thread", "change", "_user", "r:one", "secret"):
            self.assertNotIn(private, out.stdout + out.stderr)
        report = json.loads(out.stdout)["sources"][0]
        self.assertEqual(report["things"]["equal"], 4)
        self.assertEqual(report["things"]["old"], report["things"]["new"])
        self.assertEqual(report["record_count_mismatches"], 0)
        self.assertEqual([(r["facts"], r["records"]) for r in report["kinds"]], [(2, 2), (3, 3)])
        calls = [json.loads(line) for line in (self.args[2] / "calls").read_text().splitlines()]
        self.assertEqual(calls[0][0], "check")
        self.assertTrue(any(args[:2] == ["thread:", "--fields"] and
                            set(args[2].split(",")) == (set(self.kiln) | set(self.missing)) - {"odd,key"}
                            for _, args, _ in calls))
        cache, = {p for _, _, p in calls}
        self.assertFalse(Path(cache).exists())
        given = self.tmp / "caller-cache"
        self.run_compare(env={"LOOPS_CACHE_DIR": str(given)})
        self.assertEqual(json.loads((self.args[2] / "calls").read_text().splitlines()[-1])[2],
                         str(given))

    def test_values_unset_conflict_typed_equality_and_bounded_examples(self):
        changed = {**self.kiln, "status": "secret-different", "odd,key": 9, "extra": 2}
        del changed["temp"]
        del changed["_user"]
        self.things["thread:kiln"] = thing("thread", changed, heads=2, conflicts=["_user"], records=2)
        self.things["r:two"]["values"] = {**self.changes, "nested": [1, {"heat": True}]}
        self.reading()
        out = self.run_compare("--show", "1", code=1)
        report = json.loads(out.stdout)
        counts = report["sources"][0]["things"]
        self.assertEqual((counts["differing"], counts["equal"], counts["fields_differing"],
                          counts["fields_conflicting"], counts["multiple_heads"]), (1, 2, 5, 1, 1))
        self.assertEqual(len(report["examples"]), 1)
        self.assertNotIn("secret", out.stdout)
        report = json.loads(self.run_compare("--show", "4", code=1).stdout)
        kiln, = [e for e in report["examples"] if e["key"] == "kiln"]
        self.assertEqual(kiln["fields"], ["_user", "extra", "odd,key", "status", "temp"])

    def test_only_old_only_new_and_record_count(self):
        self.things["thread:yard"] = self.things.pop("thread:kiln")
        del self.things["r:two"]
        self.reading()
        report = json.loads(self.run_compare(code=1).stdout)["sources"][0]
        self.assertEqual((report["things"]["only_old"], report["things"]["only_new"]), (2, 1))
        self.assertEqual(report["record_count_mismatches"], 1)

    def test_per_kind_record_mismatch_with_identical_things_and_total(self):
        self.things["thread:kiln"]["records"] -= 1
        self.things["r:one"]["records"] = 2
        self.reading()
        report = json.loads(self.run_compare(code=1).stdout)["sources"][0]
        self.assertEqual((report["things"]["equal"], report["record_count_mismatches"]), (4, 2))
        self.assertEqual(sum(r["facts"] for r in report["kinds"]),
                         sum(r["records"] for r in report["kinds"]))

    def test_unnamed_both_ways(self):
        report = json.loads(self.run_compare("--unnamed", code=1).stdout)["sources"][0]
        self.assertEqual(report["things"]["only_old"], 1)
        self.things["r:moved"] = thing("moved", {"other": 0})
        self.reading()
        self.run_compare("--unnamed")
        report = json.loads(self.run_compare(code=1).stdout)["sources"][0]
        self.assertEqual((report["things"]["only_new"], report["record_count_mismatches"]), (1, 1))

    def test_old_tool_rows_compared_separately(self):
        rows = [{"kind": "thread", "key": "kiln", "payload": {**self.kiln, "_n": 22, "_ts": 1, "_first_seen": 42}},
                {"kind": "thread", "key": None, "payload": self.missing},
                {"kind": "moved", "key": "stale", "payload": {}}]
        rows.extend({"kind": "change", "key": n, "payload": self.changes} for n in ("a", "b"))
        old = self.tmp / "old.json"
        write_json(old, {"rows": rows})
        result = json.loads(self.run_compare("--old-json", old).stdout)["sources"][0]
        self.assertEqual(result["old_json_unnamed_rows"], 1)
        self.assertEqual([k["old_json"]["equal"] for k in result["kinds"]], [2, 2])
        rows[0]["payload"]["_user"] = "private-edit"
        write_json(old, {"rows": rows})
        report = json.loads(self.run_compare("--old-json", old, "--show", "2", code=1).stdout)
        self.assertEqual(report["sources"][0]["things"]["differing"], 0)
        self.assertEqual(report["examples"][0]["fields"], ["_user"])

    def test_union_orders_by_time_then_numeric_coordinate(self):
        # workshop's later arrival was backdated; its first arrival at t=1
        # wins locally by time in the combined read, until yard's t=1 late tie.
        other = {**self.kiln, "status": "yard-wins"}
        yard = source(self.tmp, "yard", [
            ("thread", 1, (1, 12), {"name": "kiln", "status": "yard-wins"}),
        ], {"thread:kiln": thing("thread", {"name": "kiln", "status": "yard-wins"})})
        kiln = source(self.tmp, "kiln", [
            ("thread", 0.25, (9, 9), {"name": "kiln", "status": "early"}),
        ], {"thread:kiln": thing("thread", {"name": "kiln", "status": "early"})})
        self.reading(union={"thread:kiln": thing("thread", other, heads=2)})
        result = json.loads(self.run_compare("--also", *yard, "--also", *kiln).stdout)
        self.assertEqual(result["union"], {"shared": 1, "differing": 0, "multiple_heads": 1})
        self.reading(union={"thread:kiln": thing("thread", self.kiln, heads=2, conflicts=["status"])})
        result = json.loads(self.run_compare("--also", *yard, code=1).stdout)
        self.assertEqual(result["union"]["differing"], 1)

    def test_failed_check_stops_and_suppresses_child_data(self):
        self.reading(fail="check")
        out = self.run_compare(code=1)
        self.assertEqual(out.stdout, "")
        self.assertNotIn("private", out.stderr)
        calls = (self.args[2] / "calls").read_text().splitlines()
        self.assertEqual([json.loads(c)[0] for c in calls], ["check"])
        self.reading(check={"altered": ["secret-segment"]})
        out = self.run_compare(code=1)
        self.assertNotIn("secret", out.stderr)

    def test_invalid_and_duplicate_old_ids(self):
        facts = self.args[0]
        header, first, second, *tail = facts.read_bytes().splitlines(keepends=True)
        row = json.loads(second)
        for old_id in ("", "x" * 129, "kiln yard", "kiln\n", "café", json.loads(first)["id"]):
            with self.subTest(old_id=old_id):
                row["id"] = old_id
                facts.write_bytes(header + first + json.dumps(row).encode() + b"\n" + b"".join(tail))
                self.assertIn("shadow-source-invalid at line 3:", self.run_compare(code=1).stderr)

    def test_damaged_cut_and_usage(self):
        path = self.args[0]
        path.write_bytes(path.read_bytes().replace(b"secret-open", b"secret-shut"))
        self.assertIn("trailer", self.run_compare(code=1).stderr)
        self.run_compare("--show", "-1", code=2)
