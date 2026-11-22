//! What single emits and opted-in changesets fill in at write time.
//! A plain changeset states its own envelope fields.
use crate::{
    dir::Held,
    line::{Line, hash, record_from},
};
use loops::{
    batch::{self, Draft},
    json::{Json, canon_set},
    live,
    occurred::Claim,
    record::{Record, id_text},
    stored, things,
};
use std::collections::HashMap;

/// Where a draft's `sets` comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sets {
    /// The draft states its `sets`, empty included, or states that it is
    /// read whole and so has none.
    Stated,
    /// The draft states nothing, so an emit of a thing the writer holds a
    /// record of sets the keys its payload names.
    Named,
}

/// Where a draft's `about` comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum About {
    /// The draft states its whole `about`, empty included.
    Stated,
    /// The draft states none, so it is copied from the heads it builds on.
    Copied,
}

/// Where a draft's `occurred` comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Occurred {
    /// The draft states its occurrence or explicitly leaves it absent.
    Stated,
    /// Copy the occurrence when every head states the same text.
    Copied,
}

/// One draft and the choices its writer made before envelope decoding.
#[derive(Clone, Debug)]
pub struct Statement {
    /// The unchecked draft, including any batch label references.
    pub draft: Draft,
    /// Whether to copy the heads' mentions.
    pub about: About,
    /// Whether to copy an agreed occurrence.
    pub occurred: Occurred,
    /// Whether to derive the keys from an object payload.
    pub sets: Sets,
}

/// Fill one statement against the writer's lookup and settled, admitted realm.
/// # Errors
/// Returns the inference refusals from [`of`].
pub fn one(
    view: &impl Fn(&str) -> Option<batch::Held>,
    realm: &[Record],
    statement: Statement,
) -> Result<Draft, batch::Error> {
    let draft = of(view, statement.draft)?;
    let draft = sets(realm, draft, statement.sets);
    Ok(anchor(realm, draft, statement.about, statement.occurred))
}

/// Visit a line's prerequisites first, in input order among siblings. Finish
/// the entire graph check before any line can refuse during filling.
fn preparation_order(rows: &[Statement]) -> Result<Vec<usize>, batch::Error> {
    batch::check_label_names(rows.iter().map(|s| &s.draft))?;
    let labels: HashMap<_, _> = rows
        .iter()
        .enumerate()
        .filter_map(|(i, s)| Some((s.draft.label.as_deref()?, i)))
        .collect();
    let mut active = vec![false; rows.len()];
    let mut done = vec![false; rows.len()];
    let mut order = Vec::with_capacity(rows.len());
    for i in 0..rows.len() {
        let mut stack = vec![(i, false)];
        while let Some((line, finish)) = stack.pop() {
            if done[line] {
                continue;
            }
            if finish {
                done[line] = true;
                order.push(line);
                continue;
            }
            if active[line] {
                return Err(batch::Error::LabelCycle);
            }
            active[line] = true;
            stack.push((line, true));
            let mut deps: Vec<_> = rows[line]
                .draft
                .refs()
                .iter()
                .map(|name| labels[name.as_str()])
                .collect();
            deps.sort_unstable();
            deps.dedup();
            stack.extend(deps.into_iter().rev().map(|dep| (dep, false)));
        }
    }
    Ok(order)
}

/// Prepare a changeset in memory, using each filled id in later references.
/// The result is in input order for the final batch writer and its target checks.
/// # Errors
/// Label refusals precede per-line inference and writer field refusals.
pub fn changeset(held: &Held, rows: &[Statement]) -> Result<Vec<Draft>, batch::Error> {
    let order = preparation_order(rows)?;
    let mut records: Vec<_> = held.lines().iter().map(Line::record).collect();
    let mut staged = HashMap::<String, batch::Held>::new();
    let mut labels = HashMap::<&str, String>::new();
    let mut filled = Vec::with_capacity(rows.len());
    for i in order {
        let mut row = rows[i].clone();
        row.draft.env = batch::subst_env(&row.draft.env, |name| labels.get(name).cloned())
            .ok_or(batch::Error::LabelUnknown)?;
        let view = |id: &str| staged.get(id).cloned().or_else(|| held.writer_view(id));
        let draft = one(&view, &things::realm(&records, |_, _| true), row)?;
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
    Ok(filled.into_iter().map(|(_, draft)| draft).collect())
}

/// A revision written without `of` is of what it replaces: the thing of each
/// `prev` target, which is the target's `of`, or its id when it is a root
/// (Lean `Loops.envThing`). The checks the batch would make decide when it
/// can't: a target `held` lacks is `prev-unknown`, and targets of different
/// things are `prev-unrelated`. A label is left to the batch.
///
/// # Errors
/// Returns `prev-unknown` or `prev-unrelated` when `of` cannot be filled in.
pub fn of(
    held: &impl Fn(&str) -> Option<batch::Held>,
    mut draft: Draft,
) -> Result<Draft, batch::Error> {
    if draft.env.of.is_some()
        || draft.env.prev.is_empty()
        || draft.env.prev.iter().any(|p| batch::is_label(p))
    {
        return Ok(draft);
    }
    let things = draft
        .env
        .prev
        .iter()
        .map(|id| held(id).map(|target| batch::env_thing(id, target.of.as_deref()).to_owned()))
        .collect::<Option<Vec<_>>>()
        .ok_or(batch::Error::PrevUnknown)?;
    match things.split_first() {
        Some((thing, rest)) if rest.iter().all(|t| t == thing) => {
            draft.env.of = Some(thing.clone());
            Ok(draft)
        }
        _ => Err(batch::Error::PrevUnrelated),
    }
}

/// With [`Sets::Named`], an emit of a thing `realm` holds a record of sets
/// the top-level keys its payload names, in canonical order: none names an
/// empty `sets`, a pure join. A root, and the first record of a thing `realm`
/// holds nothing of, keep no `sets` and are read whole, as is a payload that
/// is not an object.
pub fn sets(realm: &[Record], mut draft: Draft, sets: Sets) -> Draft {
    if sets == Sets::Stated || draft.env.sets.is_some() {
        return draft;
    }
    let Some(thing) = draft.env.of.as_deref().filter(|t| !batch::is_label(t)) else {
        return draft;
    };
    if !realm.iter().any(|r| r.is_of(thing)) {
        return draft;
    }
    if let Json::Obj(members) = &draft.payload {
        let names: Vec<String> = members.iter().map(|(name, _)| name.clone()).collect();
        draft.env.sets = Some(canon_set(&names));
    }
    draft
}

/// The anchor rule, the whole `about` and the shared `occurred`, over the records
/// the writer holds as a reader sees them: settled, then admitted.
///
/// An emit of a thing names in `prev`, beside what it already names, every
/// record of that thing in `realm` that is live for some key, the envelope
/// among them (`loops::live::live_in`), so every head too. An erased or
/// withheld record keeps its envelope and is named like any other. A root,
/// and the first record of a thing `realm` holds nothing of, name nothing.
///
/// With [`About::Copied`], `about` is the union of the mentions of the
/// thing's heads, every one of which the emit now builds on.
/// With [`Occurred::Copied`], the emit carries `occurred` when every head
/// states the same text.
pub fn anchor(realm: &[Record], mut draft: Draft, about: About, occurred: Occurred) -> Draft {
    let Some(thing) = draft.env.of.clone().filter(|t| !batch::is_label(t)) else {
        return draft;
    };
    let state = live::LiveState::new(realm.iter().filter(|r| r.is_of(&thing)).collect());
    draft.env.prev.extend(state.live().map(|r| id_text(&r.id)));
    draft.env.prev = canon_set(&draft.env.prev);
    if about == About::Stated && occurred == Occurred::Stated {
        return draft;
    }
    let heads = state.heads();
    if about == About::Copied {
        let mentions: Vec<String> = heads.iter().flat_map(|r| r.about.iter().cloned()).collect();
        draft.env.about = canon_set(&mentions);
    }
    if occurred == Occurred::Copied {
        let mut claims = heads.iter().map(|r| r.occurred.as_ref().map(Claim::text));
        draft.env.occurred = claims
            .next()
            .flatten()
            .filter(|text| claims.all(|other| other == Some(*text)))
            .map(str::to_owned);
    }
    draft
}

#[cfg(test)]
mod tests {
    use super::*;
    use loops::record::Digest;

    #[test]
    fn prepare_transitive_prerequisites_first_in_input_order() {
        let row = |label: &str, mentions: &[&str]| Statement {
            draft: Draft {
                label: Some(label.into()),
                env: loops::envelope::Env {
                    about: mentions.iter().map(|s| (*s).into()).collect(),
                    ..Default::default()
                },
                ..Default::default()
            },
            about: About::Stated,
            occurred: Occurred::Copied,
            sets: Sets::Named,
        };
        let mut rows = vec![
            row("@first", &["@right", "@left", "@right"]),
            row("@independent", &[]),
            row("@left", &["@deep"]),
            row("@right", &[]),
            row("@deep", &[]),
        ];
        assert_eq!(preparation_order(&rows), Ok(vec![4, 2, 3, 0, 1]));
        rows.push(row("@cycle", &["@cycle"]));
        assert_eq!(preparation_order(&rows), Err(batch::Error::LabelCycle));
        rows.push(row("@unknown", &["@missing"]));
        assert_eq!(preparation_order(&rows), Err(batch::Error::LabelUnknown));
        rows.push(row("@first", &[]));
        assert_eq!(preparation_order(&rows), Err(batch::Error::LabelDuplicate));
    }

    #[test]
    fn occurred_requires_the_same_text_on_every_head() {
        let head = |id, text: Option<&str>| Record {
            id: Digest([id; 32]),
            of: Some("task:1".into()),
            occurred: text.map(|s| Claim::of(s).expect("valid occurrence")),
            ..Record::default()
        };
        for (left, right, expected) in [
            (Some("2009?"), Some("2009?"), Some("2009?")),
            (Some("2009?"), Some("2009"), None),
            (Some("2009"), None, None),
            (None, Some("2009"), None),
            (None, None, None),
        ] {
            let realm = [head(1, left), head(2, right)];
            let mut draft = Draft::default();
            draft.env.of.clone_from(&realm[0].of);
            let filled = anchor(&realm, draft, About::Stated, Occurred::Copied);
            assert_eq!(
                filled.env.occurred.as_deref(),
                expected,
                "{left:?} {right:?}"
            );
        }
    }
}
