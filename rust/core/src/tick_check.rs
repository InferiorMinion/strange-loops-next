//! Checking a tick against one holding, as in model/Loops/TickCheck.lean.
use std::collections::HashSet;

use crate::{
    json::Json,
    record::{Digest, Record, Shown, withholding},
    result::{result_hash, result_hash_of, result_text},
    spec::{SlotValue, Spec},
    spec_hash::Rules,
    things::{realm, settle},
    tick::{TickMembers, TickReadError, fault, read_tick_members},
    where_::{Refusal, RefusalCode},
};

/// The subject envelope's kind and the opening available to the checker.
#[derive(Clone, Copy, Debug)]
pub struct TickSubject<'a> {
    /// Kind from the subject's envelope.
    pub kind: &'a str,
    /// A held opening, including JSON null if that is what the record contains.
    pub payload: Option<&'a Json>,
}

/// Counts of distinct covered ids for which replay lacks evidence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MissingInputs {
    /// Covered ids with no local form.
    pub records: usize,
    /// Held covered ids requiring a locally unavailable opening.
    pub openings: usize,
}

/// Evidence preventing replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayIssue {
    /// A covered eraser names an undeclared covered target.
    IgnoredErasure,
    /// The holding lacks records or required openings.
    Missing(MissingInputs),
}

/// Today's diagnostic for a claim under older rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TodayReport {
    /// Today's result bytes match the raw claim.
    Same,
    /// Today's result bytes differ.
    Another,
    /// Today's parser refuses the spec vocabulary.
    UnreadableSpec,
    /// The spec differs from today's parsed form.
    NoncanonicalSpec,
    /// Replay lacks the required evidence or erasure declaration.
    Unavailable(ReplayIssue),
}

/// The verdict on a tick in this holding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TickAnswer {
    /// A structural fault or an evidenced omission of a covered erasure.
    NotTick(Refusal),
    /// The subject's opening is unavailable.
    PayloadUnheld,
    /// The checker cannot parse the spec.
    UnreadableSpec,
    /// The claim uses newer rules.
    Newer,
    /// The claim uses older rules, with today's diagnostic.
    Older(TodayReport),
    /// Required inputs are missing.
    Missing(MissingInputs),
    /// The replay reproduces the claimed bytes.
    Same,
    /// The replay differs in bytes from the claim.
    Differs {
        /// Result hash of the raw claim.
        claimed: String,
        /// Result hash of the replay.
        computed: String,
    },
}

/// Closure of the covered set under prev and under links.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverClosure {
    /// Every covered envelope is held and all its links are covered.
    Yes,
    /// A held covered envelope links outside the cover.
    No,
    /// The available envelopes do not decide closure.
    Unknown,
}

/// The verdict and the two reports supported by the available evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickCheck {
    /// Verdict on the stated claim.
    pub answer: TickAnswer,
    /// Closure of the validated cover, when decidable.
    pub closure: CoverClosure,
    /// Taken ids outside the cover after whole-holding settlement and admission.
    pub taken_outside: Option<usize>,
}

/// Run the spec on covered forms after masking every declared withheld id and settling.
pub fn tick_run(
    sp: &Spec,
    covers: &[Digest],
    withheld: &[Digest],
    forms: &[Record],
) -> Vec<(String, SlotValue)> {
    let covers: HashSet<_> = covers.iter().collect();
    let withheld: HashSet<_> = withheld.iter().collect();
    let masked: Vec<_> = forms
        .iter()
        .filter(|r| covers.contains(&r.id))
        .map(|r| withholding(|r| withheld.contains(&r.id), r))
        .collect();
    sp.run(&settle(&masked))
}

// Compute tickOpen for all ids together when making a tick or counting missing openings.
pub(crate) fn open_ids(forms: &[Record]) -> HashSet<Digest> {
    let mut open: HashSet<_> = forms
        .iter()
        .filter(|r| r.shown == Shown::Here)
        .map(|r| r.id)
        .collect();
    for id in forms.iter().flat_map(|r| &r.erases) {
        open.remove(id);
    }
    open
}

/// Whether this holding has an opening for the id and no eraser of it.
pub fn tick_open(forms: &[Record], id: &Digest) -> bool {
    forms.iter().any(|r| r.id == *id && r.shown == Shown::Here)
        && !crate::things::erased_in(forms, id)
}

impl TickMembers<'_> {
    fn covers_id(&self, id: &Digest) -> bool {
        self.covers().binary_search(id).is_ok()
    }
    fn withholds_id(&self, id: &Digest) -> bool {
        self.withheld().binary_search(id).is_ok()
    }
}

fn tick_closure(m: &TickMembers<'_>, forms: &[Record]) -> CoverClosure {
    let mut held = HashSet::new();
    for r in forms.iter().filter(|r| m.covers_id(&r.id)) {
        if r.prev.iter().chain(&r.under).any(|id| !m.covers_id(id)) {
            return CoverClosure::No;
        }
        held.insert(r.id);
    }
    if held.len() == m.covers().len() {
        CoverClosure::Yes
    } else {
        CoverClosure::Unknown
    }
}

fn taken_outside(sp: &Spec, m: &TickMembers<'_>, forms: &[Record]) -> usize {
    realm(forms, |_, _| true)
        .iter()
        .filter(|r| !m.covers_id(&r.id) && sp.takes(r))
        .count()
}

fn tick_missing(m: &TickMembers<'_>, forms: &[Record]) -> MissingInputs {
    let held: HashSet<_> = forms.iter().map(|r| r.id).collect();
    let open = open_ids(forms);
    let mut missing = MissingInputs::default();
    for id in m.covers() {
        if !held.contains(id) {
            missing.records += 1;
        } else if !m.withholds_id(id) && !open.contains(id) {
            missing.openings += 1;
        }
    }
    missing
}

fn replay_issue(m: &TickMembers<'_>, forms: &[Record]) -> Option<ReplayIssue> {
    if forms
        .iter()
        .filter(|r| m.covers_id(&r.id))
        .flat_map(|r| &r.erases)
        .any(|id| m.covers_id(id) && !m.withholds_id(id))
    {
        return Some(ReplayIssue::IgnoredErasure);
    }
    let missing = tick_missing(m, forms);
    (missing != MissingInputs::default()).then_some(ReplayIssue::Missing(missing))
}

enum TickComparison {
    Same,
    Differs { claimed: String, computed: String },
}

fn replay_tick(
    h: &impl Fn(&[u8]) -> Vec<u8>,
    m: &TickMembers<'_>,
    sp: &Spec,
    forms: &[Record],
) -> Result<TickComparison, ReplayIssue> {
    if let Some(issue) = replay_issue(m, forms) {
        return Err(issue);
    }
    let computed = tick_run(sp, m.covers(), m.withheld(), forms);
    if m.result().canon() == result_text(&computed) {
        Ok(TickComparison::Same)
    } else {
        Ok(TickComparison::Differs {
            claimed: result_hash_of(h, m.result()),
            computed: result_hash(h, &computed),
        })
    }
}

fn check_members(
    h: &impl Fn(&[u8]) -> Vec<u8>,
    current: &Rules,
    m: &TickMembers<'_>,
    forms: &[Record],
) -> TickCheck {
    let closure = tick_closure(m, forms);
    let report = |answer, taken_outside| TickCheck {
        answer,
        closure,
        taken_outside,
    };
    if current < m.rules() {
        return report(TickAnswer::Newer, None);
    }
    let older = m.rules() < current;
    let spec = match m.read_spec() {
        Ok(spec) => spec,
        Err(TickReadError::Invalid(f)) => {
            return report(
                if older {
                    TickAnswer::Older(TodayReport::NoncanonicalSpec)
                } else {
                    TickAnswer::NotTick(f)
                },
                None,
            );
        }
        Err(_) => {
            return report(
                if older {
                    TickAnswer::Older(TodayReport::UnreadableSpec)
                } else {
                    TickAnswer::UnreadableSpec
                },
                None,
            );
        }
    };
    let taken = Some(taken_outside(&spec, m, forms));
    if older {
        let today = match replay_tick(h, m, &spec, forms) {
            Ok(TickComparison::Same) => TodayReport::Same,
            Ok(TickComparison::Differs { .. }) => TodayReport::Another,
            Err(issue) => TodayReport::Unavailable(issue),
        };
        return report(TickAnswer::Older(today), taken);
    }
    if let Err(f) = m.read_slots(&spec) {
        return report(TickAnswer::NotTick(f), taken);
    }
    let answer = match replay_tick(h, m, &spec, forms) {
        Ok(TickComparison::Same) => TickAnswer::Same,
        Ok(TickComparison::Differs { claimed, computed }) => {
            TickAnswer::Differs { claimed, computed }
        }
        Err(ReplayIssue::IgnoredErasure) => {
            TickAnswer::NotTick(fault(RefusalCode::TickWithheldErased, "withheld"))
        }
        Err(ReplayIssue::Missing(missing)) => TickAnswer::Missing(missing),
    };
    report(answer, taken)
}

/// Check a subject against the checker's whole holding under its rules version.
pub fn check_tick(
    h: &impl Fn(&[u8]) -> Vec<u8>,
    current: &Rules,
    subject: TickSubject<'_>,
    forms: &[Record],
) -> TickCheck {
    let answer = if subject.kind != "tick" {
        TickAnswer::NotTick(fault(RefusalCode::TickKind, "kind"))
    } else if let Some(payload) = subject.payload {
        match read_tick_members(h, current, payload) {
            Ok(m) => return check_members(h, current, &m, forms),
            Err(f) => TickAnswer::NotTick(f),
        }
    } else {
        TickAnswer::PayloadUnheld
    };
    TickCheck {
        answer,
        closure: CoverClosure::Unknown,
        taken_outside: None,
    }
}
