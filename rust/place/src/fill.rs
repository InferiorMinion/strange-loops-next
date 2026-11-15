//! What `loops emit` fills in on the single-record path, before the batch
//! writer checks the draft: `of`, the anchor and `about`.
//!
//! The protocol never infers any of them. Each is written into the envelope
//! at write time, and nothing reads it in later. A changeset (`loops emit -`)
//! states its own `of`, `prev` and `about`, and nothing here touches it.
use loops::{
    batch::{self, Draft},
    json::canon_set,
    live,
    record::{Record, id_text},
};

/// Where a draft's `about` comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum About {
    /// The draft states its whole `about`, empty included.
    Stated,
    /// The draft states none, so it is copied from the heads it builds on.
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

/// The anchor rule and the whole `about`, over `realm`, the records the
/// writer holds as a reader sees them: settled, then admitted.
///
/// An emit of a thing names in `prev`, beside what it already names, every
/// record of that thing in `realm` that is live for some key, the envelope
/// among them (`loops::live::live_in`), so every head too. An erased or
/// withheld record keeps its envelope and is named like any other. A root,
/// and the first record of a thing `realm` holds nothing of, name nothing.
///
/// With [`About::Copied`], `about` is the union of the mentions of the
/// thing's heads, every one of which the emit now builds on.
pub fn anchor(realm: &[Record], mut draft: Draft, about: About) -> Draft {
    let Some(thing) = draft.env.of.clone().filter(|t| !batch::is_label(t)) else {
        return draft;
    };
    let records = live::records_of(&thing, realm);
    let live: Vec<&Record> = records.iter().filter(|r| live::live(&records, r)).collect();
    draft.env.prev.extend(live.iter().map(|r| id_text(&r.id)));
    draft.env.prev = canon_set(&draft.env.prev);
    if about == About::Copied {
        let mentions: Vec<String> = live
            .iter()
            .filter(|r| live::live_env(&records, r))
            .flat_map(|r| r.about.iter().cloned())
            .collect();
        draft.env.about = canon_set(&mentions);
    }
    draft
}
