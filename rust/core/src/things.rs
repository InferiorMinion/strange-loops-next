//! Settlement and admission from model/Loops/Things.lean; per-thing state from model/Loops/Fold.lean.
use crate::{
    fold::{LatestBy, ordered_forms},
    json::{canon_set, str_cmp},
    record::{Digest, Record, Shown, id_text},
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    error::Error,
    fmt,
};

impl Record {
    /// Lean: `Loops.Record.thing`, model/Loops/Fold.lean.
    /// Returns the one thing this record is an emit of: `of`, read as written,
    /// or without `of` the record's own text id. `about` plays no part.
    pub fn thing(&self) -> String {
        self.of.clone().unwrap_or_else(|| id_text(&self.id))
    }

    /// Gives what Lean `Loops.Record.isOf` gives. Compares the written reference or streams the root digest digits.
    /// Checks whether the reference names this record's thing.
    pub fn is_of(&self, thing: &str) -> bool {
        self.of.as_ref().map_or_else(
            || {
                thing
                    .strip_prefix("r:")
                    .is_some_and(|s| crate::base32::matches(&self.id.0, s))
            },
            |s| s == thing,
        )
    }
}

/// Gives what Lean `Loops.shares` gives. Compares the envelopes directly.
/// Checks whether two records are of the same thing.
pub fn shares(a: &Record, b: &Record) -> bool {
    match (&a.of, &b.of) {
        (Some(t), _) => b.is_of(t),
        (None, Some(t)) => a.is_of(t),
        (None, None) => a.id == b.id,
    }
}

/// Gives what Lean `Loops.idsOf` gives. Sorts and deduplicates the digests.
/// Returns each held digest once in ascending byte order.
pub fn ids_of(xs: &[Record]) -> Vec<Digest> {
    let mut ids: Vec<_> = xs.iter().map(|r| r.id).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
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

/// Gives what Lean `Loops.settle` gives. Indexes the first preferred form and all erasure targets in one pass.
/// Keeps one form per held id, in ascending digest order.
/// Order invariance requires `Forms`, as `settle_eq` states in model/Laws/Things.lean.
/// Conflicting unchecked forms keep their first here form, or their first envelope otherwise.
pub fn settle(xs: &[Record]) -> Vec<Record> {
    let mut forms = BTreeMap::<Digest, &Record>::new();
    let mut erased = HashSet::new();
    for r in xs {
        erased.extend(r.erases.iter().copied());
        let selected = forms.entry(r.id).or_insert(r);
        if selected.shown != Shown::Here && r.shown == Shown::Here {
            *selected = r;
        }
    }
    forms
        .into_values()
        .map(|r| {
            if erased.contains(&r.id) {
                r.erase()
            } else if r.shown == Shown::Here {
                r.clone()
            } else {
                r.withhold()
            }
        })
        .collect()
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

/// Gives what Lean `Loops.admit` gives. Indexes every original form by id before testing predecessors in input order.
/// Filters once against the original held records, not against a changing accepted set.
/// The caller must supply a pure `EnvOnly` rule that depends only on both envelopes.
pub fn admit(xs: &[Record], may: impl Fn(&Record, &Record) -> bool) -> Vec<Record> {
    let mut forms = HashMap::<Digest, Vec<&Record>>::new();
    for r in xs {
        forms.entry(r.id).or_default().push(r);
    }
    xs.iter()
        .filter(|r| {
            r.prev.iter().all(|p| {
                forms
                    .get(p)
                    .is_none_or(|qs| qs.iter().all(|q| shares(r, q) && may(r, q)))
            })
        })
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

/// Gives what Lean `Loops.stateIn` gives. Collects named ids before sorting the surviving forms.
/// Folds supersession over the records of one thing, before any payload selection.
pub fn state_in(thing: &str, xs: &[Record]) -> LatestBy {
    state(&xs.iter().filter(|r| r.is_of(thing)).collect::<Vec<_>>())
}

/// Gives what Lean `Loops.perThing` gives. Partitions the records once before computing each state.
/// Returns each thing with all its current heads.
pub fn per_thing(xs: &[Record]) -> Vec<(String, LatestBy)> {
    by_thing(xs)
        .into_iter()
        .map(|(t, rs)| (t, state(&rs)))
        .collect()
}

pub(crate) fn by_thing(xs: &[Record]) -> Vec<(String, Vec<&Record>)> {
    let mut groups = HashMap::<String, Vec<&Record>>::new();
    for r in xs {
        groups.entry(r.thing()).or_default().push(r);
    }
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by(|a, b| str_cmp(&a.0, &b.0));
    groups
}

pub(crate) fn state(rs: &[&Record]) -> LatestBy {
    let mut named: Vec<_> = rs.iter().flat_map(|r| r.prev.iter().copied()).collect();
    named.sort_unstable();
    named.dedup();
    let heads = ordered_forms(
        rs.iter()
            .copied()
            .filter(|r| named.binary_search(&r.id).is_err())
            .cloned()
            .collect(),
    );
    LatestBy { named, heads }
}
