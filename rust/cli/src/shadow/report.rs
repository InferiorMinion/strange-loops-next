//! Attribute overlap to source rows, old identities and union things separately.
use crate::{count, object};
use loops::json::Json;
use std::collections::{BTreeMap, HashMap, HashSet};

const FIELDS: &[&str] = &[
    "facts_read",
    "written",
    "already_imported",
    "would_write",
    "unnamed_left_out",
    "old_ticks_left_out",
    "without_key",
    "empty_observer",
    "unnamed_included",
    "refused",
    "omitted_defects",
    "shared_old_ids",
    "shared_old_ids_one_record",
    "shared_old_ids_diverged",
    "shared_old_ids_filtered",
    "things_in_multiple_stores",
    "things_with_multiple_union_heads",
    "shared_keyed_things_different_old_ids",
    "destination_reads",
];

#[derive(Default)]
pub(super) struct Counts {
    numbers: BTreeMap<&'static str, usize>,
    reasons: BTreeMap<String, usize>,
}
impl Counts {
    pub(super) fn add(&mut self, name: &'static str, n: usize) {
        *self.numbers.entry(name).or_default() += n;
    }
    pub(super) fn reject(&mut self, reason: &str) {
        self.add("refused", 1);
        *self.reasons.entry(reason.into()).or_default() += 1;
    }
    fn json(&self) -> Json {
        let mut fields: Vec<_> = FIELDS
            .iter()
            .map(|k| (*k, count(self.numbers.get(k).copied().unwrap_or(0))))
            .collect();
        fields.push((
            "refused_by_reason",
            Json::Obj(
                self.reasons
                    .iter()
                    .map(|(k, n)| (k.clone(), count(*n)))
                    .collect(),
            ),
        ));
        object(fields)
    }
}

#[derive(Default)]
pub(super) struct Report {
    pub sources: Vec<(String, Counts)>,
    pub errors: Vec<String>,
    old: HashMap<String, Vec<(usize, Option<String>)>>,
    things: HashMap<String, HashMap<usize, HashSet<String>>>,
    heads: HashMap<String, HashSet<String>>,
    superseded: HashSet<String>,
}
impl Report {
    pub(super) fn omitted(
        &mut self,
        source: usize,
        old: &str,
        defect: Option<&super::mapping::Refusal>,
    ) {
        self.sources[source].1.add("unnamed_left_out", 1);
        if defect.is_some() {
            self.sources[source].1.add("omitted_defects", 1);
        }
        self.row(source, old, None);
    }
    fn row(&mut self, source: usize, old: &str, id: Option<&str>) {
        self.old
            .entry(old.into())
            .or_default()
            .push((source, id.map(str::to_owned)));
    }
    pub(super) fn included(
        &mut self,
        source: usize,
        old: &str,
        p: &super::mapping::Prepared,
        named: Option<&super::source::Fold>,
    ) {
        self.row(source, old, Some(&p.id));
        self.thing(
            source,
            old,
            &p.id,
            p.draft.env.of.as_deref(),
            &p.draft.env.prev,
        );
        if p.missing_key {
            self.sources[source].1.add("without_key", 1);
        }
        if named.is_none() {
            self.sources[source].1.add("unnamed_included", 1);
        }
    }
    fn thing(&mut self, source: usize, old: &str, id: &str, thing: Option<&str>, prev: &[String]) {
        let t = thing.unwrap_or(id);
        self.things
            .entry(t.into())
            .or_default()
            .entry(source)
            .or_default()
            .insert(old.into());
        self.heads.entry(t.into()).or_default().insert(id.into());
        self.superseded.extend(prev.iter().cloned());
    }
    pub(super) fn problem(&mut self, source: usize, code: &str, reason: String) {
        self.sources[source].1.reject(code);
        self.errors.push(reason);
    }
    pub(super) fn json(&self) -> Json {
        let mut overlap: Vec<Counts> = (0..self.sources.len()).map(|_| Counts::default()).collect();
        let mut total = Counts::default();
        let mut shared = |field, owners: &[usize]| {
            total.add(field, 1);
            for i in owners {
                overlap[*i].add(field, 1);
            }
        };
        for rows in self.old.values().filter(|rs| rs.len() > 1) {
            let owners: Vec<_> = rows.iter().map(|(i, _)| *i).collect();
            shared("shared_old_ids", &owners);
            let included: Vec<_> = rows
                .iter()
                .filter_map(|(i, id)| id.as_ref().map(|_| *i))
                .collect();
            if included.len() != rows.len() {
                shared("shared_old_ids_filtered", &owners);
            }
            if included.len() > 1 {
                let distinct: HashSet<_> = rows.iter().filter_map(|(_, id)| id.as_ref()).collect();
                shared(
                    if distinct.len() == 1 {
                        "shared_old_ids_one_record"
                    } else {
                        "shared_old_ids_diverged"
                    },
                    &included,
                );
            }
        }
        for (thing, by_source) in self.things.iter().filter(|(_, s)| s.len() > 1) {
            let owners: Vec<_> = by_source.keys().copied().collect();
            shared("things_in_multiple_stores", &owners);
            if self.heads[thing]
                .difference(&self.superseded)
                .take(2)
                .count()
                > 1
            {
                shared("things_with_multiple_union_heads", &owners);
            }
            if !thing.starts_with("r:")
                && by_source.values().any(|ids| ids != &by_source[&owners[0]])
            {
                shared("shared_keyed_things_different_old_ids", &owners);
            }
        }
        let sources = self
            .sources
            .iter()
            .zip(overlap)
            .map(|((name, counts), mut local)| {
                for (k, n) in &counts.numbers {
                    total.add(k, *n);
                    local.add(k, *n);
                }
                for (k, n) in &counts.reasons {
                    *total.reasons.entry(k.clone()).or_default() += n;
                }
                local.reasons.clone_from(&counts.reasons);
                (name.clone(), local.json())
            })
            .collect();
        object(vec![
            ("sources", Json::Obj(sources)),
            ("group", total.json()),
        ])
    }
}
