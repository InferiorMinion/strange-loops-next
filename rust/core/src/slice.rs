//! Select the held IDs that preserve a spec's answer, following
//! `SpecSlice` and `Spec.run_slice_keyed` in model/Laws.
use crate::{
    record::{Digest, Record},
    spec::{Follow, Spec},
};
use std::collections::{HashMap, HashSet};

/// Which ancestors to include in addition to the spec's slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closure {
    /// Keep the spec slice, including the whole-thing cover for keyed slots.
    Exact,
    /// Also include every held record reachable through `prev` and `under`.
    Downward,
}

/// Return the selected IDs in the order of `settled`, a holding with one
/// record per ID **before admission**, or a cut of that holding. All links
/// are interpreted within this input. Slot filters play no part in selection.
///
/// Include every source-taken record, its erasers, its direct predecessors
/// and successors, and each successor's direct predecessors. If any slot
/// follows keys, also include every record of each source-taken thing, its
/// erasers and its direct predecessors.
pub fn spec_slice(spec: &Spec, settled: &[Record], closure: Closure) -> Vec<Digest> {
    let keyed = spec
        .slots
        .iter()
        .any(|slot| slot.fold_type.follow() == Some(Follow::Keys));
    let mut taken = HashSet::new();
    let mut touched = HashSet::new();
    for record in settled.iter().filter(|r| spec.takes(r)) {
        taken.insert(record.id);
        if keyed {
            touched.insert(record.thing());
        }
    }
    if taken.is_empty() {
        return Vec::new();
    }

    // The ordinary slice owes forms, erasers and direct targets to each
    // taken ID. A whole-thing cover owes the same to every touched member.
    let mut members = taken.clone();
    if !touched.is_empty() {
        members.extend(
            settled
                .iter()
                .filter(|r| touched.contains(&r.thing()))
                .map(|r| r.id),
        );
    }

    let mut selected = HashSet::new();
    for record in settled {
        // Only taken IDs seed the reverse prev relation. A successor's
        // other targets decide its admission even when it is of another kind.
        let successor = record.prev.iter().any(|id| taken.contains(id));
        if successor || members.contains(&record.id) {
            selected.insert(record.id);
            selected.extend(record.prev.iter().copied());
        }
        if record.erases.iter().any(|id| members.contains(id)) {
            selected.insert(record.id);
        }
    }

    if closure == Closure::Downward {
        close_downward(settled, &mut selected);
    }
    settled
        .iter()
        .map(|r| r.id)
        .filter(|id| selected.contains(id))
        .collect()
}

fn close_downward(settled: &[Record], selected: &mut HashSet<Digest>) {
    let by_id: HashMap<_, _> = settled.iter().map(|r| (r.id, r)).collect();
    let mut pending: Vec<_> = selected.iter().copied().collect();
    while let Some(id) = pending.pop() {
        if let Some(record) = by_id.get(&id) {
            for target in record.prev.iter().chain(&record.under) {
                if selected.insert(*target) {
                    pending.push(*target);
                }
            }
        }
    }
}
