//! Index the forms common to every row (the holding or selection, then staged
//! rows). A selected base can also bring in held ancestors for just one row.
use super::{BaseNotice, Statement, anchor, sets};
use crate::dir::Held;
use loops::{
    batch::{self, Draft},
    record::{Digest, Record, Shown, id_text},
    things,
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct Forms {
    records: Vec<Record>,
    // Keep even nonpreferred forms here: their prev links seed base closure.
    by_thing: HashMap<String, Vec<usize>>,
    preferred: HashMap<Digest, usize>,
    erased: HashSet<Digest>,
    // Notice membership stays fixed as prepared rows extend the index.
    shown: Option<HashSet<String>>,
}

impl Forms {
    pub(super) fn new(held: &Held, selected: Option<&[Record]>) -> Self {
        let mut forms = Self::default();
        if let Some(base) = selected {
            forms.shown = Some(base.iter().map(|r| id_text(&r.id)).collect());
            for r in base {
                forms.insert(r.clone());
            }
        } else {
            for line in held.lines() {
                forms.insert(line.record());
            }
        }
        forms
    }

    pub(super) fn insert(&mut self, r: Record) {
        let i = self.records.len();
        self.by_thing.entry(r.thing()).or_default().push(i);
        self.erased.extend(&r.erases);
        let chosen = self.preferred.entry(r.id).or_insert(i);
        if *chosen != i && self.records[*chosen].shown != Shown::Here && r.shown == Shown::Here {
            *chosen = i;
        }
        self.records.push(r);
    }

    fn ancestors(
        &self,
        held: &Held,
        thing: &str,
        local: &[usize],
        stated: &[String],
    ) -> Vec<Record> {
        let mut todo = stated.to_vec();
        for &i in local {
            todo.extend(self.records[i].prev.iter().map(id_text));
        }
        let mut seen = HashSet::new();
        let mut extra = Vec::new();
        while let Some(id) = todo.pop() {
            if !seen.insert(id.clone()) {
                continue;
            }
            if let Some(line) = held.line(&id) {
                let r = line.record();
                if !self.preferred.contains_key(&r.id) && r.is_of(thing) {
                    todo.extend(r.prev.iter().map(id_text));
                    extra.push(r);
                }
            }
        }
        extra
    }

    fn realm(&self, local: &[usize], extra: &[Record]) -> Vec<Record> {
        let extra_erased: HashSet<_> = extra
            .iter()
            .flat_map(|r| r.erases.iter().copied())
            .collect();
        let preferred = local.iter().filter_map(|&i| {
            let r = &self.records[i];
            (self.preferred[&r.id] == i).then_some(r)
        });
        let mut realm: Vec<_> = preferred
            .chain(extra)
            .filter(|r| {
                // Admission sees all preferred targets, even refused ones. Any
                // additional ancestor already shares this thing by construction.
                r.prev.iter().all(|p| {
                    self.preferred
                        .get(p)
                        .is_none_or(|&i| things::shares(r, &self.records[i]))
                })
            })
            .map(|r| {
                if self.erased.contains(&r.id) || extra_erased.contains(&r.id) {
                    r.erase()
                } else if r.shown == Shown::Here {
                    r.clone()
                } else {
                    r.withhold()
                }
            })
            .collect();
        realm.sort_unstable_by_key(|r| r.id);
        realm
    }

    pub(super) fn fill(&self, held: &Held, row: Statement, notices: &mut Vec<BaseNotice>) -> Draft {
        let Some(thing) = row.draft.env.of.as_deref().filter(|t| !batch::is_label(t)) else {
            return row.draft;
        };
        let local = self.by_thing.get(thing).map_or(&[][..], Vec::as_slice);
        let extra = if self.shown.is_some() {
            self.ancestors(held, thing, local, &row.draft.env.prev)
        } else {
            Vec::new()
        };
        let realm = self.realm(local, &extra);
        if self.shown.is_some() && realm.is_empty() {
            notices.push(if local.is_empty() && extra.is_empty() {
                BaseNotice::Empty(thing.to_owned())
            } else {
                BaseNotice::Unadmitted(thing.to_owned())
            });
        }
        let draft = sets(&realm, row.draft, row.sets);
        let draft = anchor(&realm, draft, row.about, row.occurred);
        if let Some(shown) = &self.shown {
            let n = draft
                .env
                .prev
                .iter()
                .filter(|p| !shown.contains(*p))
                .count();
            if n > 0 {
                notices.push(BaseNotice::Named(n));
            }
        }
        draft
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fill only copies envelope fields, so comparing drafts alone cannot
    // detect an erased payload that was mistakenly left visible.
    #[test]
    fn thing_realm_matches_settlement_after_each_form() {
        let record = |id, thing: &str| Record {
            id: Digest([id; 32]),
            of: Some(thing.into()),
            ..Record::default()
        };
        let first = record(1, "thing:a");
        let mut eraser = record(2, "thing:b");
        eraser.prev.push(first.id);
        eraser.erases.push(first.id);
        let replacement = record(1, "thing:b");
        let mut ancestor = record(3, "thing:b");
        ancestor.erases.push(eraser.id);
        let mut forms = Forms::default();
        let mut raw = Vec::new();
        for r in [first.withhold(), eraser, replacement, first] {
            raw.push(r.clone());
            forms.insert(r);
            for (thing, extra) in [
                ("thing:a", &[][..]),
                ("thing:b", &[][..]),
                ("thing:b", std::slice::from_ref(&ancestor)),
            ] {
                let mut all = raw.clone();
                all.extend_from_slice(extra);
                let expected: Vec<_> = things::realm(&all, |_, _| true)
                    .into_iter()
                    .filter(|r| r.is_of(thing))
                    .collect();
                let local = forms.by_thing.get(thing).map_or(&[][..], Vec::as_slice);
                assert_eq!(forms.realm(local, extra), expected);
            }
        }
    }
}
