//! Compare the filled writer with independent, successive single writes.
mod common;
#[path = "common/statements.rs"]
mod statements;
use statements::{Sample, single, statement};

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

const BASE_SEED: u64 = 0x2026_1122_00ba_5e11;
const BASE_KEYS: [&str; 4] = ["port", "owner", "title", "extra"];

fn base_row(rng: &mut Rng, key: &Key, thing: &str, step: usize, names: &[&str]) -> Statement {
    let fields = names
        .iter()
        .map(|k| ((*k).into(), Json::Str(format!("{k}-{step}"))))
        .collect();
    Statement {
        draft: Draft {
            env: Env {
                kind: "task".into(),
                observer: key.public(),
                of: Some(thing.into()),
                emitted: emitted(step),
                ..Env::default()
            },
            salt: [rng.next().to_be_bytes(), rng.next().to_be_bytes()].concat(),
            label: None,
            payload: Json::Obj(fields),
        },
        about: About::Copied,
        occurred: Occurred::Copied,
        sets: Sets::Named,
    }
}

fn base_history(rng: &mut Rng, key: &Key, thing: &str, context: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    for step in 0..(4 + rng.below(10)) {
        let target = if step < 2 || rng.below(3) > 0 {
            thing
        } else {
            "other:task"
        };
        let name = rng.pick(&BASE_KEYS);
        let keys: &[&str] = match step {
            0 => &["port", "owner"],
            1 => &["port"],
            _ => &[name],
        };
        let mut row = base_row(rng, key, target, step, keys);
        row.draft.env.sets = Some(keys.iter().map(|k| (*k).into()).collect());
        if step > 1 {
            match rng.below(6) {
                0 => row.draft.env.sets = None,
                1 => row.draft.env.sets = Some(Vec::new()),
                _ => {}
            }
            row.sets = Sets::Stated;
        }
        if rng.below(3) == 0 {
            row.about = About::Stated;
            row.draft.env.about.push(format!("project:{step}"));
        }
        // Some writers held only an earlier prefix, giving genuine forks.
        let seen = if step > 1 && rng.below(4) == 0 {
            rng.below(step)
        } else {
            step
        };
        let line = single(&lines[..seen], key, row).expect(context);
        lines.push(if rng.below(8) == 0 {
            line.erased()
        } else {
            line
        });
    }
    lines
}

fn base_selection(
    rng: &mut Rng,
    key: &Key,
    case: usize,
    lines: &mut Vec<Line>,
    context: &str,
) -> Vec<Line> {
    let mut selected = match rng.below(6) {
        0 => lines[1..2].to_vec(),
        1 => lines[..rng.below(lines.len() + 1)].to_vec(),
        _ => lines
            .iter()
            .filter(|_| rng.below(3) == 0)
            .cloned()
            .collect(),
    };
    if case % 7 == 0 {
        let env = Env {
            kind: "erasure".into(),
            observer: key.public(),
            emitted: emitted(18),
            erases: vec![lines[case % lines.len()].id().into()],
            ..Env::default()
        };
        let written = loops::stored::write(env, vec![42; 16], Json::Null).expect(context);
        let eraser = Line::sign(&written, key);
        if case % 2 == 0 {
            selected.push(eraser.clone());
        }
        lines.push(eraser);
    }
    selected
}

// Independent reference: rescan the held edges until no id is added.
fn effective(lines: &[Line], selected: &[Line], thing: &str) -> Vec<Line> {
    let mut result = selected.to_vec();
    loop {
        let mut more = Vec::new();
        for line in lines {
            if line.record().is_of(thing)
                && !result.iter().any(|r| r.id() == line.id())
                && result
                    .iter()
                    .any(|r| r.record().is_of(thing) && r.env().prev.iter().any(|p| p == line.id()))
            {
                more.push(line.clone());
            }
        }
        if more.is_empty() {
            return result;
        }
        result.extend(more);
    }
}

fn slice_reads(thing: &str, realm: &[loops::record::Record], context: &str) {
    let live = loops::live::live_in(realm, thing);
    assert_eq!(
        loops::live::keyed_in(thing, &live),
        loops::live::keyed_in(thing, realm),
        "{context}"
    );
}

fn staged_pair(
    held: &Held,
    closure: &[Line],
    selected: &[loops::record::Record],
    mut first: Statement,
    key: &Key,
    context: &str,
) {
    let mut next = first.clone();
    first.draft.label = Some("@first".into());
    next.draft.env.of = None;
    next.draft.env.prev = vec!["@first".into()];
    next.draft.env.emitted = emitted(21);
    let (rows, _) = fill::changeset_with_base(held, &[first.clone(), next.clone()], Some(selected))
        .expect(context);
    let written = batch::resolve_batch(&hash, &|id| held.writer_view(id), &rows).expect(context);
    let original = single(closure, key, first).expect(context);
    assert_eq!(Line::sign(&written[0], key), original, "{context}");
    next.draft.env.prev = vec![original.id().into()];
    let mut seen = closure.to_vec();
    seen.push(original);
    assert_eq!(
        Line::sign(&written[1], key),
        single(&seen, key, next).expect(context),
        "{context}"
    );
}

// The bare comparison holds the flag selection plus the stated predecessor,
// before following any edges. Exercise both inferred and stated of.
fn stated_selection(
    rng: &mut Rng,
    thing: &str,
    lines: &[Line],
    selected: &[Line],
    row: &mut Statement,
) -> Vec<Line> {
    let mut bare = selected.to_vec();
    let inside = match rng.below(3) {
        0 => return bare,
        1 => true,
        _ => false,
    };
    let candidates: Vec<_> = lines
        .iter()
        .filter(|line| {
            line.record().is_of(thing) && inside == selected.iter().any(|s| s.id() == line.id())
        })
        .collect();
    if !candidates.is_empty() {
        let target = candidates[rng.below(candidates.len())];
        row.draft.env.prev.push(target.id().into());
        if rng.below(2) == 0 {
            row.draft.env.of = None;
        }
        if !inside {
            bare.push(target.clone());
        }
    }
    bare
}

#[test]
fn random_selected_bases_preserve_the_whole_and_its_live_slice() {
    use loops::{live, record::Record};
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("emit-base-random");
    let _ = fs::remove_dir_all(&root);
    let mut rng = Rng(BASE_SEED);
    let key = Key::from_text(&"0".repeat(52)).expect("key");
    let (mut unclosed, mut witnesses, mut outside_prev, mut outside_closure) = (0, 0, 0, 0);
    let mut unclosed_as_supplied = 0;
    for case in 0..CASES {
        let context = format!("seed {BASE_SEED:#x}, case {case}");
        let thing = format!("task:{}", rng.below(5));
        let mut lines = base_history(&mut rng, &key, &thing, &context);
        let selected = base_selection(&mut rng, &key, case, &mut lines, &context);
        let name = rng.pick(&BASE_KEYS);
        let mut row = base_row(&mut rng, &key, &thing, 20, &[name]);
        let bare = stated_selection(&mut rng, &thing, &lines, &selected, &mut row);
        let original = effective(&lines, &selected, &thing);
        unclosed_as_supplied += usize::from(original.len() > selected.len());
        if bare.len() > selected.len() {
            outside_prev += 1;
            outside_closure +=
                usize::from(!original.iter().any(|r| r.id() == row.draft.env.prev[0]));
        }
        let closure = effective(&lines, &bare, &thing);
        unclosed += usize::from(closure.len() > bare.len());
        let records = |lines: &[Line]| lines.iter().map(Line::record).collect::<Vec<_>>();
        let supplied = records(&selected);
        let part = live::records_of(&thing, &records(&closure));
        // A seed replaced one key of an omitted predecessor that still sets another.
        witnesses += usize::from(part.iter().any(|x| {
            !bare.iter().any(|b| b.id() == id_text(&x.id))
                && x.declares().iter().any(|k| live::live_for(&part, k, x))
                && bare.iter().any(|b| {
                    let y = b.record();
                    y.prev.contains(&x.id) && x.declares().iter().any(|k| y.sets_key(k))
                })
        }));
        let place = Dir::init(&root.join(case.to_string())).expect(&context);
        place.append(&lines).expect(&context);
        let held = place.read().expect(&context);
        if case % 4 == 0 {
            staged_pair(&held, &closure, &supplied, row.clone(), &key, &context);
        }
        let (draft, notice) = fill::one_with_base(&held, &supplied, row.clone()).expect(&context);
        let written =
            batch::resolve_batch(&hash, &|id| held.writer_view(id), &[draft]).expect(&context);
        let actual = Line::sign(&written[0], &key);
        let reference = single(&closure, &key, row.clone()).expect(&context);
        assert_eq!(actual, reference, "{context}");
        let n = reference
            .env()
            .prev
            .iter()
            .filter(|id| !selected.iter().any(|s| s.id() == *id))
            .count();
        let expected_notice = if part.is_empty() {
            vec![fill::BaseNotice::Empty(thing.clone())]
        } else if n > 0 {
            vec![fill::BaseNotice::Named(n)]
        } else {
            vec![]
        };
        assert_eq!(notice, expected_notice, "{context}");

        let world = records(&lines);
        slice_reads(&thing, &things::realm(&world, |_, _| true), &context);
        let after = |line: &Line| {
            let mut forms = world.clone();
            forms.push(line.record());
            things::realm(&forms, |_, _| true)
        };
        let closed = after(&actual);
        slice_reads(&thing, &closed, &context);
        let bare_write = single(&bare, &key, row).expect(&context);
        let other = after(&bare_write);
        // Different prev can give the new write two different ids. Match that
        // one setter across the two runs and compare all values and visibility.
        let readings = |realm: &[Record], id: Digest, k: &str| {
            live::setters(&live::records_of(&thing, realm), k)
                .into_iter()
                .map(|r| ((r.id != id).then_some(r.id), r.shown, r.fields))
                .collect::<Vec<_>>()
        };
        // An undeclared key also tests the live whole-read setters.
        for k in BASE_KEYS.into_iter().chain(["unnamed"]) {
            assert_eq!(
                readings(&closed, actual.record().id, k),
                readings(&other, bare_write.record().id, k),
                "{context}, {k}"
            );
        }
    }
    assert!(
        witnesses >= 30
            && witnesses <= unclosed
            && unclosed < CASES
            && outside_closure >= 30
            && outside_prev < CASES,
        "seed {BASE_SEED:#x}: {witnesses}, {unclosed}, {outside_prev}, {outside_closure}"
    );
    eprintln!(
        "seed {BASE_SEED:#x}: {CASES} cases, {unclosed_as_supplied} unclosed as supplied, {unclosed} unclosed with stated prev, {witnesses} witness-shaped, {outside_prev} outside prev ({outside_closure} outside original closure)"
    );
}
