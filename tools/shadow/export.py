"""Export one legacy vertex and its SQLite snapshot into a private directory."""

import argparse
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import struct
import sys
from tempfile import TemporaryDirectory

from old_id import valid_old_id
from vertex import InvalidVertex, parse_vertex


def encode_line(obj):
    return (json.dumps(obj, ensure_ascii=False, separators=(",", ":"),
                       allow_nan=False) + "\n").encode("utf-8")


def export_rows(db, output, mapping):
    with closing(sqlite3.connect(db, uri=True)) as conn:
        conn.execute("PRAGMA query_only=ON")
        conn.execute("BEGIN")
        columns = {row[1] for row in conn.execute("PRAGMA table_info(facts)")}
        required = {"id", "kind", "ts", "observer", "origin", "payload", "signature"}
        if not required <= columns:
            raise ValueError("shadow-source-invalid: missing facts columns")
        axes = ("arrival_ordinal", "arrival_seq")
        if (axes[0] in columns) != (axes[1] in columns):
            raise ValueError("shadow-source-invalid: partial arrival coordinate schema")
        order = "arrival" if axes[0] in columns else "rowid"
        axis = ", ".join(axes) if order == "arrival" else "rowid"
        ticks, = conn.execute("SELECT count(*) FROM ticks").fetchone()
        header = encode_line({"format": "shadow-facts-v1", "source": mapping["source"],
                              "vertex_sha256": mapping["vertex_sha256"], "order": order,
                              "ticks": str(ticks)})
        digest = hashlib.sha256(header).digest()
        output.write(header)
        ids = set()
        previous = None
        count = 0
        query = f"SELECT id, kind, ts, observer, origin, payload, signature, {axis} FROM facts ORDER BY {axis}"
        for fact_id, kind, ts, observer, origin, payload, signature, *coord in conn.execute(query):
            if not all(type(n) is int for n in coord) or (previous is not None and coord <= previous):
                raise ValueError(f"shadow-source-invalid: invalid arrival coordinate at row {count + 1}")
            previous = coord
            if not valid_old_id(fact_id) or fact_id in ids:
                raise ValueError(f"shadow-source-invalid at line {count + 2}: invalid or duplicate old id")
            ids.add(fact_id)
            if (not all(isinstance(s, str) for s in (kind, observer, origin, payload))
                    or not isinstance(ts, (int, float))
                    or (signature is not None and not isinstance(signature, str))):
                raise ValueError(f"shadow-source-invalid: invalid fact columns at row {count + 1}")
            row = encode_line({"id": fact_id, "kind": kind, "ts_bits": struct.pack(">d", ts).hex(),
                               "observer": observer, "origin": origin, "payload": payload,
                               "signature": signature, "coordinate": [str(n) for n in coord]})
            output.write(row)
            digest = hashlib.sha256(digest + row).digest()
            count += 1
        output.write(encode_line({"count": str(count), "digest": digest.hex()}))


def export(vertex, out, quiescent_copy=False):
    raw = vertex.read_bytes()
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise InvalidVertex(raw[:error.start].count(b"\n") + 1, "expected UTF-8") from error
    mapping = parse_vertex(text)
    mapping["vertex_sha256"] = hashlib.sha256(raw).hexdigest()
    db = (vertex.parent / mapping["store"]).resolve(strict=True)
    wal = Path(str(db) + "-wal")
    if db.parent == out.resolve():
        raise ValueError("shadow-source-invalid: select an output directory apart from the database")
    has_wal = wal.exists()
    if has_wal and not quiescent_copy:
        raise ValueError("shadow-source-invalid: WAL present, supply a quiescent copy")
    journal = Path(str(db) + "-journal")
    out.mkdir(mode=0o700, parents=True, exist_ok=True)
    with TemporaryDirectory(dir=out) as temp:
        stage = Path(temp)
        if has_wal or journal.exists():
            # Give SQLite private copies of any sidecars it needs to consult.
            replica = stage.resolve() / "snapshot"
            shutil.copyfile(db, replica)
            if has_wal:
                shutil.copyfile(wal, stage / "snapshot-wal")
            if journal.exists():
                shutil.copyfile(journal, stage / "snapshot-journal")
            uri = replica.as_uri() + "?mode=ro"
        else:
            # The input is a frozen snapshot. immutable also prevents SQLite from
            # creating sidecars for a checkpointed database still in WAL mode.
            uri = db.as_uri() + "?mode=ro&immutable=1"
        facts = stage / (mapping["source"] + ".facts.jsonl")
        reviewed = stage / (mapping["source"] + ".map.json")
        with facts.open("wb") as output:
            try:
                export_rows(uri, output, mapping)
            except sqlite3.Error as error:
                raise ValueError(f"shadow-source-invalid: {error}") from error
        reviewed.write_bytes(encode_line(mapping))
        for path in (facts, reviewed):
            path.chmod(0o600)
        # No overwrite, including when two exporters publish to the same directory.
        os.link(reviewed, out / reviewed.name)
        try:
            os.link(facts, out / facts.name)
        except OSError:
            (out / reviewed.name).unlink()
            raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("vertex", type=Path)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--quiescent-copy", action="store_true",
                        help="the main file and its WAL are a frozen, consistent pair")
    args = parser.parse_args()
    try:
        export(args.vertex, args.output_dir.resolve(), args.quiescent_copy)
    except (ValueError, OSError, sqlite3.Error) as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
