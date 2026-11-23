"""Walk two invented SQLite snapshots through export, shadow-import and comparison."""

from contextlib import closing
import json
from pathlib import Path
import sqlite3
import subprocess
import sys


def main(loops, tools):
    uuid = "bb17229c-f77a-4ec3-a7e8-6c6ed4024d70"
    sources = []
    for name in ("workshop", "yard"):
        vertex = Path(name + ".vertex")
        vertex.write_text(f'name "{name}"\nstore "{name}.sqlite"\n'
                          'loops { change { fold { items "collect" 4 } } }\n', encoding="utf-8")
        with closing(sqlite3.connect(name + ".sqlite")) as db:
            db.execute("CREATE TABLE ticks(id TEXT)")
            db.execute("CREATE TABLE facts(id TEXT, kind TEXT, ts REAL, observer TEXT, "
                       "origin TEXT, payload TEXT, signature TEXT)")
            ids = [uuid]
            if name == "workshop":
                ids.extend(["0" * 26, "!", "~" * 128])
            for old_id in ids:
                db.execute("INSERT INTO facts VALUES (?, 'change', 1.5, 'potter', '', '{}', NULL)",
                           (old_id,))
            db.commit()
        subprocess.run(["python3", tools / "export.py", vertex, "--output-dir", "cut"], check=True)
        sources.append(dict(name=name, facts=f"cut/{name}.facts.jsonl", map=f"cut/{name}.map.json",
                            destination=name))
        subprocess.run([loops, "-C", name, "init"], check=True)

    Path("group.json").write_text(json.dumps(dict(state="private", sources=sources)), encoding="utf-8")
    report = json.loads(subprocess.check_output([loops, "shadow-import", "group.json"]))
    assert report["group"]["shared_old_ids_one_record"] == 1
    workshop, yard = (subprocess.check_output([loops, "-C", name, "export"]).splitlines()
                     for name in ("workshop", "yard"))
    assert len(workshop) == 4 and len(yard) == 1 and yard[0] in workshop
    for src in sources:
        subprocess.run(["python3", tools / "compare.py", src["facts"], src["map"],
                        src["destination"], "--loops", loops], check=True)


if __name__ == "__main__":
    main(sys.argv[1], Path(sys.argv[2]))
