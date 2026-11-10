//! Settlement and admission from model/Loops/Things.lean; per-thing state from model/Loops/Fold.lean.
use crate::{
    fold::{LatestBy, insert_id},
    json::canon_set,
    record::{Digest, Record, Shown, id_text},
};
use std::{error::Error, fmt};

impl Record {
    /// Lean: `Loops.Record.thing`, model/Loops/Fold.lean.
    /// Returns the one thing this record is an emit of: `of`, read as written,
    /// or without `of` the record's own text id. `about` plays no part.
    pub fn thing(&self) -> String {
        self.of.clone().unwrap_or_else(|| id_text(&self.id))
    }

    /// Lean: `Loops.Record.isOf`, model/Loops/Fold.lean.
    /// Checks whether the reference names this record's thing.
    pub fn is_of(&self, thing: &str) -> bool {
        self.thing() == thing
    }
}

/// Lean: `Loops.shares`, model/Loops/Fold.lean.
/// Checks whether two records are of the same thing.
pub fn shares(a: &Record, b: &Record) -> bool {
    a.thing() == b.thing()
}

/// Lean: `Loops.idsOf`, model/Loops/Things.lean.
/// Returns each held digest once in ascending byte order.
pub fn ids_of(xs: &[Record]) -> Vec<Digest> {
    xs.iter().fold(Vec::new(), |ids, r| insert_id(r.id, &ids))
}

/// Lean: `Loops.erasedIn`, model/Loops/Things.lean.
/// Checks every original form's erasure links, regardless of kind or admission.
pub fn erased_in(xs: &[Record], id: &Digest) -> bool {
    xs.iter().any(|r| r.erases.contains(id))
}

/// Lean: `Loops.baseForm`, model/Loops/Things.lean.
/// Keeps the first here form, or withholds the first form when no payload is here.
/// This function does not check ids or the `Forms` premise.
pub fn base_form(forms: &[Record]) -> Option<Record> {
    forms
        .iter()
        .find(|r| r.shown == Shown::Here)
        .cloned()
        .or_else(|| forms.first().map(Record::withhold))
}

/// Lean: `Loops.pick`, model/Loops/Things.lean.
/// Chooses a base form for this id, then applies erasure links from every original form.
pub fn pick(xs: &[Record], id: &Digest) -> Option<Record> {
    let forms: Vec<_> = xs.iter().filter(|r| r.id == *id).cloned().collect();
    base_form(&forms).map(|r| if erased_in(xs, id) { r.erase() } else { r })
}

/// Lean: `Loops.settle`, model/Loops/Things.lean.
/// Keeps one form per held id, in ascending digest order.
/// Order invariance requires `Forms`, as `settle_eq` states in model/Laws/Things.lean.
/// Conflicting unchecked forms keep their first here form, or their first envelope otherwise.
pub fn settle(xs: &[Record]) -> Vec<Record> {
    ids_of(xs).iter().filter_map(|id| pick(xs, id)).collect()
}

/// Lean: `Loops.entitled`, model/Loops/Things.lean.
/// Checks all held targets of every succession link.
/// Missing targets pass. Every matching target must be of the record's thing and pass `may`.
/// The caller must supply a pure `EnvOnly` rule that depends only on both envelopes.
pub fn entitled(xs: &[Record], r: &Record, may: impl Fn(&Record, &Record) -> bool) -> bool {
    r.prev
        .iter()
        .all(|p| xs.iter().all(|q| q.id != *p || (shares(r, q) && may(r, q))))
}

/// Lean: `Loops.refusalOf`, model/Loops/Things.lean.
/// These two codes describe succession links that fail admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A held predecessor is of a different thing from the record.
    PrevUnrelated,
    /// The rule denies replacement of a held predecessor.
    PrevNotPermitted,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::PrevUnrelated => "prev-unrelated",
            Self::PrevNotPermitted => "prev-not-permitted",
        })
    }
}
impl Error for Refusal {}

/// Lean: `Loops.refusalOf`, model/Loops/Things.lean.
/// Checks all links for unrelated predecessors before checking any permission.
/// Missing targets pass. Every matching held form must be of the record's thing and pass `may`.
/// The caller supplies a pure `EnvOnly` rule for the admission laws.
pub fn refusal_of(
    xs: &[Record],
    r: &Record,
    may: impl Fn(&Record, &Record) -> bool,
) -> Option<Refusal> {
    if !r
        .prev
        .iter()
        .all(|p| xs.iter().all(|q| q.id != *p || shares(r, q)))
    {
        Some(Refusal::PrevUnrelated)
    } else if !entitled(xs, r, may) {
        Some(Refusal::PrevNotPermitted)
    } else {
        None
    }
}

/// Lean: `Loops.admit`, model/Loops/Things.lean.
/// Filters once against the original held records, not against a changing accepted set.
/// The caller must supply a pure `EnvOnly` rule that depends only on both envelopes.
pub fn admit(xs: &[Record], may: impl Fn(&Record, &Record) -> bool) -> Vec<Record> {
    xs.iter()
        .filter(|r| entitled(xs, r, &may))
        .cloned()
        .collect()
}

/// Lean: `Loops.realm`, model/Loops/Things.lean.
/// Settles all forms before admission, including erasure links from records that admission later refuses.
/// The caller supplies a pure `EnvOnly` rule for the admission laws.
pub fn realm(xs: &[Record], may: impl Fn(&Record, &Record) -> bool) -> Vec<Record> {
    admit(&settle(xs), may)
}

/// Lean: `Loops.thingsOf`, model/Loops/Fold.lean.
/// Returns the thing of every record, each once, in canonical UTF-16 order.
pub fn things_of(xs: &[Record]) -> Vec<String> {
    canon_set(&xs.iter().map(Record::thing).collect::<Vec<_>>())
}

/// Lean: `Loops.stateIn`, model/Loops/Fold.lean.
/// Folds supersession over the records of one thing, before any payload selection.
pub fn state_in(thing: &str, xs: &[Record]) -> LatestBy {
    let mut state = LatestBy::default();
    state.fold(
        &xs.iter()
            .filter(|r| r.is_of(thing))
            .cloned()
            .collect::<Vec<_>>(),
    );
    state
}

/// Lean: `Loops.perThing`, model/Loops/Fold.lean.
/// Returns each thing with all its current heads.
pub fn per_thing(xs: &[Record]) -> Vec<(String, LatestBy)> {
    things_of(xs)
        .into_iter()
        .map(|t| {
            let state = state_in(&t, xs);
            (t, state)
        })
        .collect()
}
