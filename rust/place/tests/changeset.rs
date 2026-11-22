//! Compare the filled writer with independent, successive single writes.
mod common;

use common::{Rng, emitted};
use loops::{
    batch::{self, Draft},
    envelope::Env,
    json::Json,
    record::{Digest, id_text},
    things,
};
use loops_place::{
    dir::{Dir, Held},
    fill::{self, About, Occurred, Sets, Statement},
    key::Key,
    line::{Line, hash},
};
use std::{fs, path::Path};

const SEED: u64 = 0x2026_1120_d8c0_0005;
const CASES: usize = 320;

fn single(lines: &[Line], key: &Key, row: Statement) -> Result<Line, batch::Error> {
    let view = |id: &str| {
        lines
            .iter()
            .find(|line| line.id() == id)
            .map(|line| batch::Held {
                of: line.env().of.clone(),
                prev: line.env().prev.clone(),
            })
    };
    let draft = fill::of(&view, row.draft)?;
    let records: Vec<_> = lines.iter().map(Line::record).collect();
    let realm = things::realm(&records, |_, _| true);
    let draft = fill::sets(&realm, draft, row.sets);
    let draft = fill::anchor(&realm, draft, row.about, row.occurred);
    let written = batch::resolve_batch(&hash, &view, &[draft])?;
    Ok(Line::sign(&written[0], key))
}

fn changeset(
    place: &Dir,
    held: &Held,
    key: &Key,
    rows: &[Statement],
) -> Result<Vec<Line>, batch::Error> {
    let filled = fill::changeset(held, rows)?;
    let written = batch::resolve_batch(&hash, &|id| held.writer_view(id), &filled)?;
    let lines: Vec<_> = written.iter().map(|w| Line::sign(w, key)).collect();
    place.append(&lines).expect("append one segment");
    Ok(lines)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sample {
    Holding,
    Changeset,
}

fn statement(rng: &mut Rng, key: &Key, lines: &[Line], step: usize, sample: Sample) -> Statement {
    let thing = format!("task:{}", rng.below(3));
    let mut env = Env {
        kind: "task".into(),
        observer: key.public(),
        emitted: emitted(step),
        of: Some(thing.clone()),
        ..Env::default()
    };
    let related: Vec<_> = lines
        .iter()
        .filter(|line| batch::env_thing(line.id(), line.env().of.as_deref()) == thing)
        .collect();
    if rng.below(9) == 0 {
        env.kind = "erasure".into();
        env.of = None;
        if !lines.is_empty() && rng.below(4) != 0 {
            env.erases
                .push(lines[rng.below(lines.len())].id().to_owned());
        }
    } else {
        if !related.is_empty() && rng.below(2) == 0 {
            env.prev
                .push(related[rng.below(related.len())].id().to_owned());
        }
        if rng.below(4) == 0 {
            env.of = None;
        }
    }
    let about = match rng.below(4) {
        0 => About::Stated,
        1 => {
            env.about.push(rng.pick(&["project:a", "project:b"]).into());
            About::Stated
        }
        _ => About::Copied,
    };
    let occurred = match rng.below(4) {
        0 => Occurred::Stated,
        1 => {
            env.occurred = Some(rng.pick(&["2009", "2010?"]).into());
            Occurred::Stated
        }
        _ => Occurred::Copied,
    };
    let sets = match rng.below(5) {
        0 => Sets::Stated, // whole
        1 => {
            env.sets = Some(vec![]);
            Sets::Stated
        }
        2 => {
            env.sets = Some(vec!["x".into()]);
            Sets::Stated
        }
        _ => Sets::Named,
    };
    if sample == Sample::Changeset {
        match rng.below(20) {
            0 => env.prev.push(id_text(&Digest([0xff; 32]))),
            1 if !lines.is_empty() => env.prev.push(lines[rng.below(lines.len())].id().into()),
            2 => env.kind.clear(),
            _ => {}
        }
    }
    let payload = Json::Obj(vec![(
        rng.pick(&["x", "y", "z"]).into(),
        Json::Str(step.to_string()),
    )]);
    let salt = [rng.next().to_be_bytes(), rng.next().to_be_bytes()].concat();
    Statement {
        draft: Draft {
            label: None,
            env,
            salt,
            payload,
        },
        about,
        occurred,
        sets,
    }
}

#[test]
fn filled_changesets_match_successive_emits() {
    let mut rng = Rng(SEED);
    let key = Key::from_text(&"0".repeat(52)).expect("fixed key");
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("changeset-equivalence");
    let _ = fs::remove_dir_all(&root);
    let mut accepted = 0;
    let mut different_codes = 0;
    for case in 0..CASES {
        let context = format!("seed {SEED:#x}, case {case}");
        let case_dir = root.join(case.to_string());
        let place = Dir::init(&case_dir.join("batch")).expect("case store");
        let singles = Dir::init(&case_dir.join("singles")).expect("twin store");
        let mut lines = Vec::new();
        for step in 0..rng.below(9) {
            let row = statement(&mut rng, &key, &lines, step, Sample::Holding);
            let line = single(&lines, &key, row).expect(&context);
            lines.push(line);
        }
        place.append(&lines).expect("seed store");
        singles.append(&lines).expect("seed twin");
        let held = place.read().expect("checked holding");
        let rows: Vec<_> = (0..=rng.below(6))
            .map(|step| statement(&mut rng, &key, &lines, 10 + step, Sample::Changeset))
            .collect();
        let batch = changeset(&place, &held, &key, &rows);
        let start = lines.len();
        let seq = rows.into_iter().try_for_each(|row| {
            let line = single(&lines, &key, row)?;
            singles
                .append(std::slice::from_ref(&line))
                .expect("append single");
            lines.push(line);
            Ok(())
        });
        match seq {
            Ok(()) => {
                let got = batch.expect(&context);
                // Line equality includes the id, envelope, opening and signature.
                assert_eq!(got, lines[start..], "{context}");
                accepted += 1;
            }
            Err(code) => {
                let refused = batch.expect_err(&context);
                different_codes += usize::from(refused != code);
                let after = place.read().expect("check after refusal");
                assert_eq!(held.segments(), after.segments(), "{context}");
                assert_eq!(held.lines(), after.lines(), "{context}");
            }
        }
    }
    assert!(accepted > 0 && accepted < CASES, "seed {SEED:#x}");
    eprintln!(
        "seed {SEED:#x}: {CASES} cases, {accepted} accepted, {} refused, {different_codes} different refusal codes",
        CASES - accepted
    );
}

#[test]
fn staged_predecessor_changes_admission_and_withheld_head_still_supplies_its_envelope() {
    let key = Key::from_text(&"0".repeat(52)).expect("fixed key");
    let row = |thing: &str| Statement {
        draft: Draft {
            env: Env {
                kind: "task".into(),
                emitted: emitted(0),
                observer: key.public(),
                of: Some(thing.into()),
                ..Env::default()
            },
            salt: vec![0; 16],
            payload: Json::Obj(vec![]),
            label: None,
        },
        about: About::Copied,
        occurred: Occurred::Copied,
        sets: Sets::Named,
    };
    let predecessor = row("task:a");
    let target = single(&[], &key, predecessor.clone()).expect("future predecessor");
    // Import a signed envelope whose predecessor is missing locally. A reader
    // admits it until the differently related predecessor actually arrives.
    let mut orphan = row("task:b").draft;
    orphan.env.prev.push(target.id().into());
    let written =
        loops::stored::write(orphan.env, orphan.salt, orphan.payload).expect("checked fields");
    let orphan = Line::sign(&written, &key);
    let mut head = row("task:b");
    head.about = About::Stated;
    head.draft.env.about.push("project:p".into());
    head.occurred = Occurred::Stated;
    head.draft.env.occurred = Some("2009".into());
    let head = single(&[], &key, head).expect("head").erased();
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fill-readmission");
    let _ = fs::remove_dir_all(&path);
    let dir = Dir::init(&path).expect("store");
    let mut lines = vec![head.clone(), orphan.clone()];
    dir.append(&lines).expect("import");
    let mut infer = row("task:b");
    infer.draft.env.of = None;
    infer.draft.env.prev.push(orphan.id().into());
    let rows = [predecessor, row("task:b"), infer];
    let result = changeset(&dir, &dir.read().expect("held"), &key, &rows).expect("filled");
    for (row, staged) in rows.into_iter().zip(&result) {
        let written = single(&lines, &key, row).expect("single");
        assert_eq!(staged, &written);
        lines.push(written);
    }
    assert_eq!(result[1].env().prev, [head.id()]);
    assert_eq!(result[1].env().about, ["project:p"]);
    assert_eq!(result[1].env().occurred.as_deref(), Some("2009"));
    assert_eq!(result[2].env().of.as_deref(), Some("task:b"));
}
