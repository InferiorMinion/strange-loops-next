//! What `loops emit` fills in on the single-record path, before the batch
//! writer checks the draft: `of`, `sets`, the anchor, `about` and `occurred`.
//!
//! The protocol never infers any of them. Each is written into the envelope
//! at write time, and nothing reads it in later. A changeset (`loops emit -`)
//! states its own `of`, `sets`, `prev`, `about` and `occurred`,
//! and nothing here touches it.
use loops::{
    batch::{self, Draft},
    json::{Json, canon_set},
    live,
    occurred::Claim,
    record::{Record, id_text},
};

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
