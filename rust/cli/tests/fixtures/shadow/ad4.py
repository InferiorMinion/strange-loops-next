"""Exercise the AD4 writer through the exporter and real CLI in fresh test scratch."""
import base64
import hashlib
import os
import json
from pathlib import Path
import sqlite3
import subprocess
import sys

LOOPS, TOOLS = (str(Path(arg).resolve()) for arg in sys.argv[1:])
POTTER = "01M858Z8H3HM44556677889900"
END = "bb17229c-f77a-4ec3-a7e8-6c6ed4024d70"
LINKS = ["links", "members"]
SOURCES = ["workshop", "yard", "kiln"]


def run(*args, error=None, stdin=None):
    p = subprocess.run([LOOPS, *args], input=stdin, capture_output=True, text=True)
    if error:
        assert p.returncode == 1 and error in p.stderr, (args, p.returncode, p.stderr)
    else:
        assert p.returncode == 0, (args, p.stderr)
    return p.stdout


def imp(*args, error=None):
    return json.loads(run("shadow-import", "potter.group.json", *args, error=error))


def save(path, value):
    Path(path).write_text(json.dumps(value))


def read(path):
    return json.loads(Path(path).read_text())


def snapshot():
    files = [p for p in Path("private").glob("*") if p.is_file()]
    files += list(Path(".").glob("*/.loops/segments/*/*"))
    files += list(Path(".").glob("*.ids.json"))
    return {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in files}


def write_facts(name, rows):
    with sqlite3.connect(Path(name + ".sqlite")) as db:
        db.execute("CREATE TABLE IF NOT EXISTS ticks(id TEXT)")
        db.execute("CREATE TABLE IF NOT EXISTS facts(id TEXT, kind TEXT, ts REAL, observer TEXT, "
                   "origin TEXT, payload TEXT, signature TEXT)")
        for old, kind, observer, payload in rows:
            db.execute("INSERT INTO facts VALUES(?,?,1.5,?,'',?,NULL)",
                       (old, kind, observer, json.dumps(payload)))


def export(name, cut):
    subprocess.run([sys.executable, Path(TOOLS) / "export.py", name + ".vertex", "--output-dir", cut],
                   check=True, capture_output=True)
    mapping = Path(cut) / (name + ".map.json")
    if name == "workshop":
        data = read(mapping)
        data["ref_fields"] = LINKS.copy()
        save(mapping, data)
    return dict(name=name, destination=name, map=str(mapping), facts=f"{cut}/{name}.facts.jsonl")


def init():
    for name in SOURCES:
        Path(name).mkdir()
        run("-C", name, "init")
        Path(name + ".vertex").write_text(f'name "{name}"\nstore "{name}.sqlite"\nloops {{\n'
              'task { fold { items "by" "name" } }\nperson { fold { items "by" "name" } }\n'
              'observer { fold { items "by" "name" } }\nchange { fold { items "collect" 3 } }\n}\n')
    write_facts("workshop", [
        (POTTER, "person", "", {"name": "potter"}),
        ("shared", "change", "", {"links": POTTER}),
        ("middle", "change", "", {"ref": END}),
        ("first", "task", "potter", {"name": "kiln", "ref": "thread:firing,decision/design/fold,?,task/kiln",
             "owner": "person/potter", "owner_ref": POTTER, "loose_ref": POTTER,
             "image": "kiln", "image_ref": "sha256:abc", "links": f"{END}, {POTTER}", "members": [POTTER, END]}),
        ("second", "task", "", {"name": "kiln", "ref": "yard/root", "owner": "", "owner_ref": "",
                                "loose_ref": "", "links": [], "members": ["mixed", 7]}),
        ("third", "task", "yard", {"name": "kiln", "owner": "kiln", "owner_ref": ["second"]}),
        ("self", "task", "", {"name": "self", "ref": "task/self", "ref_ref": "self"}),
        ("cycle-a", "change", "", {"ref": "cycle-b, person/potter"}),
        ("absent", "change", "", {"ref": "omitted,defect,r:made-up,Wrong/name,task/"}),
        ("partial", "change", "", {"owner": "potter", "owner_ref": f"{POTTER},later-id"}),
        ("empty", "change", "", {"ref": "", "owner": "", "owner_ref": " , "}),
        ("odd", "change", "", {"ref": 7, "image_ref": ["kiln"]}),
        ("own-observer", "observer", "potter", {"name": "potter"}),
        ("own-root", "change", "", {"ref": "own-root"}),
        ("omitted", "unused", "", {"name": "yard"}),
        ("defect", "unused", "", []),
        ("by-address", "change", "", {"ref": "task/legacy"}),
        ("local", "change", "", {"ref": "shared"}),
    ])
    write_facts("yard", [
        (END, "change", "", {"heat": 4}),
        ("task/legacy", "task", "", {"name": "yard"}),
        ("shared", "change", "", {"links": POTTER}),
        ("cycle-b", "change", "", {"ref": "cycle-a"}),
        ("external", "change", "", {"ref": "middle,cycle-a"}),
        ("local-yard", "change", "", {"ref": "shared"}),
        ("missing-key", "task", "", {"heat": "low"}),
    ])
    write_facts("kiln", [("ambiguous", "change", "", {"ref": "shared"}),
                         ("root-reader", "change", "", {"ref": "missing-key"})])
    return {"state": "private", "sources": [export(s, "exports") for s in SOURCES]}


def records(ids, store):
    lines = {r["id"]: r for r in map(json.loads, run("-C", store, "export").splitlines())}
    result = {}
    for old, entries in ids.items():
        for e in entries:
            assert set(e) == {"source", "destination", "record", "thing"}
            if e["source"] == store:
                r = lines.pop(e["record"])
                assert e["destination"] == store and e["thing"] == r["env"].get("of", r["id"])
                result[old] = r
    assert not lines
    return result


def check_records(ids):
    w, y, k = (records(ids, s) for s in SOURCES)
    def check(row, about, fields):
        assert set(row["env"].get("about", [])) == set(about), row
        assert row["payload"] == fields, row
    end = y[END]["id"]
    check(w["first"], ["thread:firing", "decision:design/fold", "person:potter", end, "observer:potter"],
          {"name": "kiln", "ref": "thread:firing,decision/design/fold,?,task/kiln", "owner": "person/potter",
           "loose_ref": POTTER, "image": "kiln", "image_ref": "sha256:abc", "links": f"{END}, {POTTER}", "members": [POTTER, END]})
    check(w["second"], ["thread:firing", "decision:design/fold", "yard:root"],
          {"name": "kiln", "owner": "", "loose_ref": "", "links": [], "members": ["mixed", 7]})
    assert set(w["second"]["env"]["sets"]) == set(w["second"]["payload"]) | {"ref"}
    check(w["third"], ["thread:firing", "decision:design/fold", "yard:root", "observer:yard"],
          {"name": "kiln", "owner": "kiln", "owner_ref": ["second"]})
    check(w["self"], [], {"name": "self"})
    check(w["own-observer"], [], {"name": "potter"})
    check(w["own-root"], [], {})
    check(w["middle"], [end], {})
    check(y["external"], [w["middle"]["id"], w["cycle-a"]["id"]], {})
    check(w["cycle-a"], ["person:potter"], {"ref": "cycle-b, person/potter"})
    check(y["cycle-b"], [], {"ref": "cycle-a"})
    check(w["by-address"], ["task:yard"], {})
    check(w["partial"], ["person:potter"], {"owner": "potter", "owner_ref": f"{POTTER},later-id"})
    check(w["empty"], [], {"owner": "", "owner_ref": " , "})
    check(w["odd"], [], {"ref": 7, "image_ref": ["kiln"]})
    check(w["local"], [w["shared"]["id"]], {})
    check(y["local-yard"], [y["shared"]["id"]], {})
    assert len(ids["shared"]) == 2 and w["shared"]["id"] != y["shared"]["id"]
    check(k["ambiguous"], [], {"ref": "shared"})
    check(k["root-reader"], [y["missing-key"]["id"]], {})
    assert ids["omitted"] == ids["defect"] == []
    spec = {"vertex": "kiln", "from": [{"store": ".", "about": end}], "fold": {"n": {"count": True}}}
    # The raw record source sees the references on two workshop records.
    assert json.loads(run("-C", "workshop", "fold", "-", stdin=json.dumps(spec)))["n"]["value"] == 2
    assert len(run("-C", "workshop", "export", "--about", end).splitlines()) == 2


def main():
    group = init()
    manifest = Path("potter.group.json")
    save(manifest, group)
    before = snapshot()
    dry = imp("--check-only")
    assert before == snapshot() and not Path("private").exists()
    g = imp()["group"]
    full = snapshot()
    ids = read("potter.group.ids.json")
    assert Path("potter.group.ids.json").stat().st_mode & 0o777 == 0o600
    check_records(ids)
    again = imp()["group"]
    assert g["written"] == again["already_imported"] == dry["group"]["would_write"] and again["written"] == 0
    assert full == snapshot()
    print(json.dumps({"first": g, "rerun": again}, sort_keys=True))
    # Paired suffix, carried text and replaced reviewed fields are counted once per occurrence.
    expected = {"ref_parts": 23, "suffix_parts": 6, "reviewed_parts": 5, "suffix_id_hits": 4,
                "reviewed_id_hits": 5, "thing_mentions": 13, "record_mentions": 7, "self_dropped": 4,
                "self_observer_dropped": 1, "left_out_unnamed": 1, "left_out_defect": 1,
                "ambiguous_target": 1, "cycle": 2, "ref_not_text": 1, "reviewed_unsupported": 1,
                "invalid_ref_part": 4, "nonmember_id_part": 2, "ref_removed": 10, "suffix_removed": 3,
                "suffix_kept_not_text": 1, "suffix_kept_no_companion": 3, "suffix_kept_unresolved": 3}
    for field, n in expected.items():
        assert g[field] == again[field] == sum(s[field] for s in dry["sources"].values()) == n, (field, n)
    journal = [json.loads(line)[1] for line in Path("private/journal.jsonl").read_text().splitlines()]
    header = journal[0]
    assert header[0] == "v2"
    pin = next(p[2] for p in header[2] if p[0] == "workshop")
    semantic = {"kinds": read(group["sources"][0]["map"])["kinds"], "ref_fields": sorted(LINKS)}
    data = json.dumps(semantic, sort_keys=True, separators=(",", ":")).encode()
    assert pin == base64.b32hexencode(hashlib.sha256(data).digest()).decode().rstrip("=").lower()
    first_reserve = next(i for i, frame in enumerate(journal) if frame[0] == "reserve")
    seeds = [e[0] for frame in journal[:first_reserve] if frame[0] == "seeds" for e in frame[1]]
    assert len(seeds) == len(set(seeds)) and set(seeds) == {old for old, copies in ids.items() if copies}
    # Actual comment edit and re-export. Carry the review into this new cut by hand.
    with Path("workshop.vertex").open("a") as out:
        out.write("// potter reviewed the kiln\n")
    group["sources"][0] = export("workshop", "reexport")
    save(manifest, group)
    assert imp()["group"]["written"] == 0 and full == snapshot()
    mapping = Path(group["sources"][0]["map"])
    original = read(mapping)
    for kinds in [
        {**original["kinds"], "task": {"by": "potter"}},
        {**original["kinds"], "change": {"collect": "8"}},
        {**original["kinds"], "yard": {"collect": "0"}},
        {k: v for k, v in original["kinds"].items() if k != "observer"},
    ]:
        save(mapping, {**original, "kinds": kinds})
        imp(error="shadow-state-mismatch")
        assert full == snapshot()
    for fields in [None, [], LINKS + ["another"]]:
        m = {**original, "ref_fields": fields}
        if fields is None:
            del m["ref_fields"]
        save(mapping, m)
        imp(error="shadow-state-mismatch")
    for fields in ["links", [""], ["links", "links"], ["ref"], ["owner_ref"], [7]]:
        save(mapping, {**original, "ref_fields": fields})
        imp(error="shadow-map-invalid")
    save(mapping, {**original, "ref_fields": LINKS[::-1]})
    assert imp()["group"]["written"] == 0 and full == snapshot()
    write_facts("kiln", [("append", "change", "", {"ref": "middle"})])
    group["sources"][2] = export("kiln", "append")
    save(manifest, group)
    assert imp()["group"]["written"] == 1
    updated = read("potter.group.ids.json")
    assert {k: v for k, v in updated.items() if k != "append"} == ids
    assert records(updated, "kiln")["append"]["env"]["about"] == [ids["middle"][0]["record"]]
    stable = snapshot()
    write_facts("kiln", [("later-id", "person", "", {"name": "potter"})])
    group["sources"][2] = export("kiln", "drift")
    save(manifest, group)
    for flags in [(), ("--check-only",)]:
        imp(*flags, error="shadow-state-mismatch")
        assert stable == snapshot()
    # A complete checksummed header from the previous version.
    log = Path("private/journal.jsonl")
    body = json.loads(log.read_text().splitlines()[0])[1]
    body[0] = "v1"
    raw = json.dumps(body, separators=(",", ":")).encode()
    checksum = base64.b32hexencode(hashlib.sha256(raw).digest()).decode().rstrip("=").lower()
    log.write_bytes(b'["' + checksum.encode() + b'",' + raw + b']\n')
    prior = snapshot()
    for flags in [(), ("--check-only",)]:
        imp(*flags, error="v1 shadow journal; import into fresh shadows")
        assert prior == snapshot()


def different_folds(include_unnamed):
    # The keyed, collected, omitted and included-unnamed copies share exact old bytes.
    maps = {"workshop": {"item": {"by": "name"}, "keyref": {"by": "ref"}, "change": {"collect": "4"}},
            "yard": {"item": {"collect": "4"}, "potter": {"by": "name"}, "change": {"collect": "4"}},
            "kiln": {"change": {"collect": "4"}}}
    rows = {"workshop": [("item", "item", "", {"name": "kiln"}), ("p", "potter", "", {"name": "potter"}),
                         ("w", "change", "", {"ref": "item,p"}), ("same", "change", "", {}),
                         ("keyref1", "keyref", "", {"ref": "thread/kiln"}),
                         ("keyref2", "keyref", "", {"ref": "thread/kiln"}),
                         ("suffix1", "item", "", {"name": "yard", "owner": None, "owner_ref": "?"}),
                         ("suffix2", "item", "", {"name": "yard", "owner": None, "owner_ref": "p"})],
            "yard": [("item", "item", "", {"name": "kiln"}), ("p", "potter", "", {"name": "potter"}),
                     ("y", "change", "", {"ref": "item"}), ("same", "change", "", {})],
            "kiln": [("k", "change", "", {"ref": "item,same"})]}
    sources = []
    for name in SOURCES:
        Path(name).mkdir()
        run("-C", name, "init")
        kinds = [f'{kind} {{ fold {{ items "{op}" ' + (json.dumps(value) if op == "by" else value) + ' } }'
                 for kind, fold in maps[name].items() for op, value in fold.items()]
        Path(name + ".vertex").write_text(f'name "{name}"\nstore "{name}.sqlite"\nloops {{\n' +
                                           "\n".join(kinds) + "\n}\n")
        write_facts(name, rows[name])
        sources.append(export(name, "cut"))
    save("potter.group.json", {"state": "private", "sources": sources})
    g = imp(*(["--include-unnamed"] if include_unnamed else []))["group"]
    ids = read("potter.group.ids.json")
    w, y, k = (records(ids, s) for s in SOURCES)
    local_p = w["p"]["id"] if include_unnamed else "potter:potter"
    assert set(w["w"]["env"]["about"]) == {"item:kiln", local_p}
    assert y["y"]["env"]["about"] == [y["item"]["id"]]
    assert w["same"]["id"] == y["same"]["id"]
    assert k["k"]["env"]["about"] == [w["same"]["id"]] and k["k"]["payload"] == {"ref": "item,same"}
    assert g["ambiguous_target"] == 1
    assert w["keyref1"]["payload"] == w["keyref2"]["payload"] == {}
    assert w["keyref2"]["env"]["of"] == "keyref:thread/kiln" and w["keyref2"]["env"]["sets"] == []
    assert set(w["suffix2"]["env"]["sets"]) == {"owner_ref", "owner", "name"}
    assert w["suffix2"]["payload"] == {"name": "yard", "owner": None}
    # A missing id map is reconstructed only by a writable run.
    path = Path("potter.group.ids.json")
    data = path.read_bytes()
    path.unlink()
    flags = ["--include-unnamed"] if include_unnamed else []
    imp(*flags, "--check-only")
    assert not path.exists()
    assert imp(*flags)["group"]["written"] == 0 and path.read_bytes() == data


if __name__ == "__main__":
    main()
    for include in (False, True):
        directory = Path(f"fates-{include}")
        directory.mkdir()
        os.chdir(directory)
        different_folds(include)
        os.chdir("..")
