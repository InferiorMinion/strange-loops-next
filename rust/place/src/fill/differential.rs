use super::*;
use crate::{dir::Dir, fill, key::Key, line::Line};
use loops::things;
use std::collections::HashSet;

#[path = "../../tests/common/mod.rs"]
mod common;
#[path = "../../tests/common/statements.rs"]
mod statements;
use common::{Rng, emitted};
use statements::{Sample, single, statement};

const SEED: u64 = 0x2026_1123_f111_ba7c;

// Original batch algorithm: settle and admit all effective forms again for every row.
fn rescan(
    held: &Held,
    rows: &[Statement],
    selected: Option<&[Record]>,
) -> Result<(Vec<Draft>, Vec<BaseNotice>), batch::Error> {
    let mut notices = Vec::new();
    let order = preparation_order(rows)?;
    let mut records = if selected.is_some() {
        Vec::new()
    } else {
        held.lines().iter().map(Line::record).collect()
    };
    let mut staged = HashMap::<String, batch::Held>::new();
    let mut labels = HashMap::<&str, String>::new();
    let mut filled = Vec::with_capacity(rows.len());
    for i in order {
        let mut row = rows[i].clone();
        row.draft.env = batch::subst_env(&row.draft.env, |name| labels.get(name).cloned())
            .ok_or(batch::Error::LabelUnknown)?;
        let view = |id: &str| staged.get(id).cloned().or_else(|| held.writer_view(id));
        let draft = if let Some(base) = selected {
            let (draft, row_notices) = fill_base(held, base, &records, &view, row)?;
            notices.extend(row_notices);
            draft
        } else {
            one(&view, &things::realm(&records, |_, _| true), row)?
        };
        let written = stored::write(draft.env.clone(), draft.salt.clone(), draft.payload.clone())?;
        let id = stored::id_of(&hash, written.env(), &draft.commit(&hash));
        records.push(record_from(&id, written.env(), Some(written.payload())));
        if let Some(label) = rows[i].draft.label.as_deref() {
            labels.insert(label, id.clone());
        }
        staged.insert(
            id,
            batch::Held {
                of: draft.env.of.clone(),
                prev: draft.env.prev.clone(),
            },
        );
        filled.push((i, draft));
    }
    filled.sort_unstable_by_key(|(i, _)| *i);
    Ok((
        filled.into_iter().map(|(_, draft)| draft).collect(),
        notices,
    ))
}

/// Keep the selected and prepared forms for settlement and admission. Close the
/// thing's records and its stated predecessors through locally held prev links.
fn effective_forms(
    held: &Held,
    selected: &[Record],
    prepared: &[Record],
    stated: &[String],
    thing: &str,
) -> Vec<Record> {
    let mut forms = selected.to_vec();
    forms.extend_from_slice(prepared);
    let mut seen: HashSet<_> = forms.iter().map(|r| id_text(&r.id)).collect();
    let mut todo = stated.to_vec();
    for r in forms.iter().filter(|r| r.is_of(thing)) {
        todo.extend(r.prev.iter().map(id_text));
    }
    while let Some(id) = todo.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let Some(line) = held.line(&id) else {
            continue;
        };
        let record = line.record();
        if record.is_of(thing) {
            todo.extend(record.prev.iter().map(id_text));
            forms.push(record);
        }
    }
    forms
}

/// TK11 option C. The notice compares the final envelope with the selection
/// as supplied, before adding stated predecessors, earlier lines or ancestors.
fn fill_base(
    held: &Held,
    selected: &[Record],
    prepared: &[Record],
    view: &impl Fn(&str) -> Option<batch::Held>,
    mut row: Statement,
) -> Result<(Draft, Vec<BaseNotice>), batch::Error> {
    row.draft = of(view, row.draft)?;
    let Some(thing) = row.draft.env.of.as_deref().filter(|t| !batch::is_label(t)) else {
        return Ok((row.draft, Vec::new()));
    };
    let forms = effective_forms(held, selected, prepared, &row.draft.env.prev, thing);
    let realm = things::realm(&forms, |_, _| true);
    let records: Vec<_> = realm.iter().filter(|r| r.is_of(thing)).collect();
    let mut notices = Vec::new();
    if records.is_empty() {
        notices.push(if forms.iter().any(|r| r.is_of(thing)) {
            BaseNotice::Unadmitted(thing.to_owned())
        } else {
            BaseNotice::Empty(thing.to_owned())
        });
    }
    let state = live::LiveState::new(records);
    let draft = sets(&realm, row.draft, row.sets);
    let draft = anchor_state(&state, draft, row.about, row.occurred);
    let shown: HashSet<_> = selected.iter().map(|r| id_text(&r.id)).collect();
    let n = draft
        .env
        .prev
        .iter()
        .filter(|id| !shown.contains(*id))
        .count();
    if n > 0 {
        notices.push(BaseNotice::Named(n));
    }
    Ok((draft, notices))
}

// The orphan is admitted until its unrelated, missing predecessor arrives.
// In other histories that predecessor is already present without its opening.
fn history(rng: &mut Rng, key: &Key) -> (Vec<Line>, Statement) {
    let mut future = statement(rng, key, &[], 0, Sample::Holding);
    future.draft.env.of = None;
    let root = single(&[], key, future.clone()).expect("future root");
    let mut orphan = statement(rng, key, &[], 1, Sample::Holding).draft;
    orphan.env.of = Some("task:orphan".into());
    orphan.env.prev.push(root.id().into());
    let orphan = stored::write(orphan.env, orphan.salt, orphan.payload).expect("orphan");
    let mut lines = vec![Line::sign(&orphan, key)];
    if rng.below(2) == 0 {
        lines.push(root.erased());
    }
    for step in 2..(4 + rng.below(6)) {
        let row = statement(rng, key, &lines, step, Sample::Holding);
        let line = single(&lines, key, row).expect("generated holding");
        match rng.below(4) {
            0 => lines.push(line.erased()),
            1 => lines.extend([line.erased(), line]),
            _ => lines.push(line),
        }
    }
    (lines, future)
}

fn rows(
    rng: &mut Rng,
    key: &Key,
    lines: &[Line],
    future: &Statement,
    case: usize,
) -> Vec<Statement> {
    let sample = if case % 8 >= 5 {
        Sample::Changeset
    } else {
        Sample::Holding
    };
    let mut rows: Vec<_> = (0..(3 + rng.below(4)))
        .map(|i| statement(rng, key, lines, 10 + i, sample))
        .collect();
    match case % 8 {
        0 => {
            rows[2] = future.clone();
            rows[2].draft.label = Some("@root".into());
            rows[0].draft.env.of = None;
            rows[0].draft.env.prev = vec!["@root".into()];
            rows[1].draft.env.of = Some("task:orphan".into());
        }
        1 => {
            for row in &mut rows {
                row.draft.env.of = Some("task:new".into());
                row.draft.env.prev.clear();
            }
        }
        2 => {
            // A differently related prev refuses this eraser at admission.
            rows[0].draft.env.of = Some("task:eraser".into());
            rows[0].draft.env.prev = vec![lines[0].id().into()];
            rows[0].draft.env.erases = vec![lines[0].id().into()];
            rows[1].draft.env.of = Some("task:orphan".into());
            rows[2].draft.env.of = Some("task:eraser".into());
        }
        3 => {
            rows[0] = future.clone();
            rows[1].draft.env.of = Some("task:orphan".into());
            rows[2].draft.env.of = Some(lines[0].env().prev[0].clone());
        }
        4 => {
            rows[0].draft.label = Some("@first".into());
            rows[1].draft.env.about.push("@first".into());
            rows[2].draft.env.under = Some("@first".into());
        }
        _ => {}
    }
    rows
}

#[test]
fn indexed_fill_matches_per_row_realm() {
    let mut rng = Rng(SEED);
    let key = Key::from_text(&"0".repeat(52)).expect("fixed key");
    let dir = std::env::temp_dir().join(format!("loops-fill-{}", std::process::id()));
    std::fs::create_dir(&dir).expect("create fresh directory for generated holdings");
    let mut accepted = 0;
    for group in 0..64 {
        let (lines, future) = history(&mut rng, &key);
        let place = Dir::init(&dir.join(group.to_string())).expect("case store");
        place.append(&lines).expect("seed store");
        let held = place.read().expect("checked holding");
        for i in 0..32 {
            let case = group * 32 + i;
            let rows = rows(&mut rng, &key, &lines, &future, case);
            let mut selected: Vec<_> = lines
                .iter()
                .filter(|_| rng.below(3) == 0)
                .map(Line::record)
                .collect();
            if case % 7 == 0 {
                selected.clear();
            } else if case % 3 == 0 {
                // The library also accepts unchecked, conflicting base forms.
                // Exercise both first-here preference and raw closure seeds.
                let mut other = lines[0].record().withhold();
                other.of = Some("task:new".into());
                other.prev.push(lines[1].record().id);
                selected.insert(0, other);
                selected.push(lines[0].record());
            }
            for base in [None, Some(selected.as_slice())] {
                let result = changeset_with_base(&held, &rows, base);
                assert_eq!(
                    result,
                    rescan(&held, &rows, base),
                    "seed {SEED:#x}, case {case}, selected base {}",
                    base.is_some()
                );
                accepted += usize::from(result.is_ok());
            }
        }
    }
    assert!(accepted > 0 && accepted < 4096);
    std::fs::remove_dir_all(dir).expect("clean up generated holdings");
}
