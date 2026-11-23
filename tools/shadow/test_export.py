import hashlib
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest

from export import export
from vertex import InvalidVertex, parse_vertex


VERTEX = '''// kiln workshop
name "workshop"
store "workshop.sqlite"
loops {
  /* ignore this /* nested */ block */
  thread {
    fold { items "by" "name" }
    edge "tool" targets="tool"
    lifecycle "status" active="open"
    search "message"; preview "status"
    boundary every=4 { run "kiln {ready}" }
  }
  boundary after=8
  /- unused { ignored "}" }
  change { fold { items "collect" 2 } }
}
'''
PAYLOAD = ' { "message" : "café" ,  "name" : "kiln" } '
GOLDEN = Path(__file__).resolve().parents[2] / "rust/cli/tests/fixtures/shadow"


def make_cut(root, arrival):
    vertex = root / "workshop.vertex"
    vertex.write_text(VERTEX, encoding="utf-8")
    db = root / "workshop.sqlite"
    conn = sqlite3.connect(db)
    try:
        columns = ", arrival_ordinal INTEGER, arrival_seq INTEGER" if arrival else ""
        conn.execute("CREATE TABLE facts(id TEXT PRIMARY KEY, kind TEXT, ts REAL, "
                     f"observer TEXT, origin TEXT, payload TEXT, signature TEXT{columns})")
        conn.execute("CREATE TABLE ticks(id TEXT)")
        conn.execute("INSERT INTO ticks VALUES ('kiln')")
        for rowid, seq in [(11, 1), (4, 2), (30, 0)]:
            row = [rowid, f"{rowid:026d}", "thread", 1.5, "", "", PAYLOAD, None]
            if arrival:
                row.extend((5 if seq < 2 else 9, seq))
            conn.execute(f"INSERT INTO facts(rowid, id, kind, ts, observer, origin, payload, signature"
                         + (", arrival_ordinal, arrival_seq" if arrival else "")
                         + ") VALUES (" + ",".join("?" for _ in row) + ")", row)
        conn.commit()
    finally:
        conn.close()
    return vertex, db


class ExportTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.out = self.root / "shadow"

    def check_schema(self, arrival):
        vertex, db = make_cut(self.root, arrival)
        before = db.read_bytes()
        export(vertex, self.out)
        self.assertEqual(before, db.read_bytes())
        self.assertEqual(sorted(p.name for p in self.root.iterdir()),
                         ["shadow", "workshop.sqlite", "workshop.vertex"])
        facts = (self.out / "workshop.facts.jsonl").read_bytes()
        raw = facts.splitlines(keepends=True)
        header, *rows, trailer = map(json.loads, raw)
        self.assertEqual(header["order"], "arrival" if arrival else "rowid")
        self.assertEqual(header["ticks"], "1")
        self.assertEqual([r["coordinate"] for r in rows],
                         [["5","0"],["5","1"],["9","2"]] if arrival else [["4"],["11"],["30"]])
        self.assertEqual([int(r["id"]) for r in rows], [30,11,4] if arrival else [4,11,30])
        self.assertTrue(all(r["payload"].encode() == PAYLOAD.encode() for r in rows))
        self.assertTrue(all(r["ts_bits"] == "3ff8000000000000" for r in rows))
        digest = hashlib.sha256(raw[0]).digest()
        for line in raw[1:-1]:
            digest = hashlib.sha256(digest + line).digest()
        self.assertEqual(trailer, {"count": "3", "digest": digest.hex()})
        mapping = (self.out / "workshop.map.json").read_bytes()
        self.assertEqual(json.loads(mapping)["kinds"], {"thread":{"by":"name"}, "change":{"collect":"2"}})
        fixture = "workshop.facts.jsonl" if arrival else "workshop.rowid.facts.jsonl"
        self.assertEqual(facts, (GOLDEN / fixture).read_bytes())
        self.assertEqual(mapping, (GOLDEN / "workshop.map.json").read_bytes())

    def test_arrival(self):
        self.check_schema(True)

    def test_rowid(self):
        self.check_schema(False)

    def test_bad_coordinates(self):
        for mutation in ("ALTER TABLE facts RENAME COLUMN arrival_seq TO unused",
                         "UPDATE facts SET arrival_seq=NULL WHERE rowid=11",
                         "UPDATE facts SET arrival_seq=0 WHERE rowid=11"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory(dir=self.root) as root:
                vertex, db = make_cut(Path(root), True)
                conn = sqlite3.connect(db)
                conn.execute(mutation)
                conn.commit()
                conn.close()
                before = db.read_bytes()
                with self.assertRaisesRegex(ValueError, "shadow-source-invalid"):
                    export(vertex, self.out)
                self.assertEqual(before, db.read_bytes())
                self.assertEqual(list(self.out.iterdir()), [])

    def test_wal(self):
        vertex, db = make_cut(self.root, False)
        conn = sqlite3.connect(db)
        self.addCleanup(conn.close)
        conn.execute("PRAGMA journal_mode=WAL")
        conn.execute("INSERT INTO ticks VALUES ('yard')")
        conn.commit()
        # The source and its active WAL are stationary during the copy.
        snapshot = {p: p.read_bytes() for p in self.root.glob("workshop.sqlite*")}
        with self.assertRaisesRegex(ValueError, "quiescent copy"):
            export(vertex, self.out)
        export(vertex, self.out, quiescent_copy=True)
        self.assertEqual(snapshot, {p: p.read_bytes() for p in self.root.glob("workshop.sqlite*")})
        self.assertEqual(json.loads((self.out / "workshop.facts.jsonl").read_bytes().splitlines()[0])["ticks"], "2")

    def test_publication(self):
        vertex, db = make_cut(self.root, False)
        with self.assertRaisesRegex(ValueError, "apart"):
            export(vertex, db.parent)
        self.out.mkdir()
        facts = self.out / "workshop.facts.jsonl"
        facts.write_bytes(b"existing")
        with self.assertRaises(FileExistsError):
            export(vertex, self.out)
        self.assertEqual(list(self.out.iterdir()), [facts])
        self.assertEqual(facts.read_bytes(), b"existing")


class VertexTests(unittest.TestCase):
    def test_refusals_name_original_line(self):
        cases = [
            ("store \"workshop.sqlite\"", 'combine { vertex "./yard.vertex" }'),
            ("store \"workshop.sqlite\"", 'discover "./yard"'),
            ("name \"workshop\"", "unknown"),
            ("name \"workshop\"", 'name "../yard"'),
            ("name \"workshop\"", 'name "yard"; name "kiln"'),
            ("store \"workshop.sqlite\"", 'store "yard" backend="unknown"'),
            ("thread {", 'thread "arg" {'),
            ('edge "tool" targets="tool"', 'unknown "tool"'),
            ('items "by" "name"', 'size "by" "name"'),
            ('items "by" "name"', 'items "sum" "name"'),
            ('items "by" "name"', 'items "by"'),
            ('items "by" "name"', 'items "by" ""'),
            ('items "by" "name"', 'items "by" "name" { bogus }'),
            ('items "collect" 2', 'items "collect" -1'),
            ('items "collect" 2', 'items "collect" "2"'),
            ('items "collect" 2', 'items "collect" 2; items "by" "name"'),
            ('fold { items "by" "name" }', 'fold { }; fold { }'),
            ('/* ignore this /* nested */ block */', '/* never closes'),
            ('name "workshop"', 'name "unterminated'),
        ]
        for old, new in cases:
            with self.subTest(new=new):
                line = VERTEX[:VERTEX.index(old)].count("\n") + 1
                with self.assertRaisesRegex(InvalidVertex, f"at line {line}:"):
                    parse_vertex(VERTEX.replace(old, new))

    def test_missing_or_unbalanced_blocks(self):
        for vertex in ('name "yard"', VERTEX.replace('loops {', 'loops;'),
                       VERTEX.rstrip()[:-1], VERTEX + '}', VERTEX.replace('  thread {', '  thread;\n  {'),
                       VERTEX.replace('fold { items "by" "name" }', 'preview "kiln"')):
            with self.subTest(vertex=vertex), self.assertRaisesRegex(InvalidVertex, "at line"):
                parse_vertex(vertex)

    def test_comments_properties_and_quoted_values(self):
        text = VERTEX.replace('"name"', r'/-disabled="wrong" /- "skip" "n\u{61}me"')
        text = text.replace('fold { items "by"', 'fold /- { ignored "}" } { items "by"')
        self.assertEqual(parse_vertex(text)["kinds"]["thread"], {"by": "name"})
        for old, replacement in [
            ('"by" "name"', '"by""name"'),
            ('"by" "name"', r'"by" "\u{d800}"'),
            ('"by" "name"', 'mode="by" "name"'),
            ('"collect" 2', '"collect" 18446744073709551616'),
        ]:
            with self.subTest(replacement=replacement), self.assertRaisesRegex(InvalidVertex, "at line"):
                parse_vertex(VERTEX.replace(old, replacement))
