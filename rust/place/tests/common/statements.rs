use super::{About, Key, Line, Occurred, Rng, Sets, Statement, emitted, fill, hash};
use loops::{
    batch::{self, Draft},
    envelope::Env,
    json::Json,
    record::{Digest, id_text},
    things,
};

pub(super) fn single(lines: &[Line], key: &Key, row: Statement) -> Result<Line, batch::Error> {
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Sample {
    Holding,
    Changeset,
}

pub(super) fn statement(
    rng: &mut Rng,
    key: &Key,
    lines: &[Line],
    step: usize,
    sample: Sample,
) -> Statement {
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
