//! Choose the holding for a tick before running its spec.
use crate::{Fail, held_forms, one_id, usage};
use loops::{
    json::Json,
    occurred::occurred_span,
    record::{Digest, Record},
    result::record_id_of,
    time::Moment,
};
use loops_place::dir::Held;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) enum Cut {
    #[default]
    All,
    AsOf(Moment),
    Cover(Vec<String>),
}

impl Cut {
    pub(crate) fn select(&mut self, flag: &str, value: String) -> Result<(), Fail> {
        match (flag, &mut *self) {
            ("--as-of", Self::All) => {
                let span = occurred_span(&value)
                    .filter(|_| !value.contains('/'))
                    .ok_or_else(|| usage("--as-of takes an instant or a date"))?;
                *self = Self::AsOf(span.start().clone());
            }
            ("--as-of", Self::AsOf(_)) => return Err(usage("use --as-of once")),
            ("--cover", Self::Cover(_) | Self::All) => {
                if value.is_empty() || value == "r:" || value.starts_with("--") {
                    return Err(usage("--cover takes a record id or prefix"));
                }
                if let Self::Cover(ids) = self {
                    ids.push(value);
                } else {
                    *self = Self::Cover(vec![value]);
                }
            }
            _ => return Err(usage("use either --as-of or --cover")),
        }
        Ok(())
    }

    pub(crate) fn forms(self, held: &Held) -> Result<Vec<Record>, Fail> {
        let seeds = match self {
            Self::All => return Ok(held_forms(held)),
            Self::AsOf(bound) => {
                let records = held.settled();
                return Ok(as_of(records, &bound));
            }
            Self::Cover(ids) => ids
                .iter()
                .map(|given| {
                    let id = one_id(held, given)?;
                    Ok(record_id_of(&Json::Str(id)).expect("a held id is a digest"))
                })
                .collect::<Result<Vec<_>, Fail>>()?,
        };
        // Settlement over the store determines which openings the writer
        // holds, even if an eraser lies outside the cut. Admission and the
        // claim's own settlement happen inside make_tick over just these ids.
        Ok(down_of(held.settled(), seeds))
    }
}

fn as_of(records: Vec<Record>, bound: &Moment) -> Vec<Record> {
    // Cut note §2 uses the strict end of a window, and so does Lean's upTo.
    let seeds = records
        .iter()
        .filter(|r| r.emitted.moment() < bound)
        .map(|r| r.id)
        .collect();
    down_of(records, seeds)
}

// downOf W P: the seed and all held predecessors, across the whole holding.
// The settled input has one envelope per id. Preserve its order in the cut.
fn down_of(mut records: Vec<Record>, mut todo: Vec<Digest>) -> Vec<Record> {
    let by_id: HashMap<_, _> = records.iter().map(|r| (r.id, r)).collect();
    let mut covered = HashSet::new();
    while let Some(id) = todo.pop() {
        if covered.insert(id) {
            if let Some(record) = by_id.get(&id) {
                todo.extend(&record.prev);
            }
        }
    }
    records.retain(|r| covered.contains(&r.id));
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use loops::occurred::Stamp;

    fn record(id: u8, second: u8, prev: &[u8]) -> Record {
        Record {
            id: Digest([id; 32]),
            of: Some(format!("thing:{id}")),
            prev: prev.iter().map(|p| Digest([*p; 32])).collect(),
            emitted: Stamp::of(&format!("1970-01-01T00:00:{second:02}Z")).expect("stamp"),
            ..Record::default()
        }
    }

    #[test]
    fn strict_time_seed_and_cross_thing_prev_closure() {
        let a = record(1, 0, &[]);
        let b = record(2, 5, &[1]);
        let c = record(3, 1, &[2]);
        let world = vec![a.clone(), b, c];
        let at = |s| record(0, s, &[]).emitted.moment().clone();
        assert_eq!(as_of(world.clone(), &at(1)), vec![a]);
        assert_eq!(as_of(world.clone(), &at(2)), world);
    }

    #[test]
    fn repeated_and_missing_predecessors_terminate_without_following_other_links() {
        let a = record(1, 0, &[2, 2, 42]);
        let mut b = record(2, 0, &[1]);
        let other = record(3, 0, &[]);
        b.under = Some(other.id);
        b.erases.push(other.id);
        assert_eq!(
            down_of(vec![a.clone(), b.clone(), other], vec![b.id, b.id]),
            [a, b]
        );
    }
}
