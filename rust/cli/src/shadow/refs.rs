//! Extract reference-bearing parts from an original payload before cleaning it.
use super::source::Map;
use loops::{envelope::typed_ref, json::Json};
use std::collections::{BTreeMap, HashMap, HashSet};

pub(super) enum Lookup {
    Target(String),
    SelfReference,
    Nonmember,
    Unresolved(&'static str),
}

#[derive(Clone, Copy)]
enum Location {
    Ref,
    Suffix,
    Reviewed,
}

pub(super) struct Field {
    name: String,
    location: Location,
    companion: bool,
    empty_text: bool,
    parts: Option<Vec<String>>,
}

impl Field {
    pub(super) fn read(payload: &Json, map: &Map) -> Vec<Self> {
        let Json::Obj(members) = payload else {
            return Vec::new();
        };
        members
            .iter()
            .filter_map(|(name, value)| {
                let (location, companion) = if name == "ref" {
                    (Location::Ref, false)
                } else if let Some(base) = name.strip_suffix("_ref").filter(|s| !s.is_empty()) {
                    (Location::Suffix, members.iter().any(|(k, _)| k == base))
                } else if map.ref_fields.contains(name) {
                    (Location::Reviewed, false)
                } else {
                    return None;
                };
                let parts = match value {
                    Json::Str(s) => Some(split(s).collect()),
                    Json::Arr(array) if matches!(location, Location::Reviewed) => {
                        array_parts(array)
                    }
                    _ => None,
                };
                Some(Self {
                    name: name.clone(),
                    location,
                    companion,
                    empty_text: matches!(value, Json::Str(s) if s.is_empty()),
                    parts,
                })
            })
            .collect()
    }

    pub(super) fn parts(&self) -> impl Iterator<Item = &str> {
        self.parts.iter().flatten().map(String::as_str)
    }
}

fn array_parts(array: &[Json]) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    for value in array {
        let Json::Str(s) = value else { return None };
        parts.extend(split(s));
    }
    Some(parts)
}

fn split(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn address(part: &str) -> Option<String> {
    let (prefix, value) = part.split_once([':', '/'])?;
    let name = format!("{prefix}:{value}");
    (prefix != "r" && typed_ref(&name)).then_some(name)
}

#[derive(Debug, Default)]
pub(super) struct Changes {
    additions: HashSet<String>,
    replacements: HashMap<String, HashSet<String>>,
    pub removed: HashSet<String>,
    pub stats: BTreeMap<&'static str, usize>,
}

impl Changes {
    pub(super) fn count(&mut self, name: &'static str) {
        *self.stats.entry(name).or_default() += 1;
    }

    pub(super) fn resolve(
        fields: &[Field],
        own: Option<&str>,
        lookup: impl Fn(&str) -> Lookup,
    ) -> Self {
        let mut changes = Self::default();
        for field in fields {
            let mut mentions = HashSet::new();
            let mut complete = field.parts.is_some();
            for part in field.parts() {
                changes.count(match field.location {
                    Location::Ref => "ref_parts",
                    Location::Suffix => "suffix_parts",
                    Location::Reviewed => "reviewed_parts",
                });
                let mut result = lookup(part);
                if matches!(result, Lookup::Nonmember) {
                    result = match field.location {
                        Location::Ref => address(part)
                            .map_or(Lookup::Unresolved("invalid_ref_part"), Lookup::Target),
                        _ => Lookup::Unresolved("nonmember_id_part"),
                    };
                } else {
                    match field.location {
                        Location::Ref => (),
                        Location::Suffix => changes.count("suffix_id_hits"),
                        Location::Reviewed => changes.count("reviewed_id_hits"),
                    }
                }
                match result {
                    Lookup::Target(t) if Some(t.as_str()) != own => {
                        mentions.insert(t);
                    }
                    Lookup::Target(_) | Lookup::SelfReference => changes.count("self_dropped"),
                    Lookup::Unresolved(reason) => {
                        changes.count(reason);
                        complete = false;
                    }
                    Lookup::Nonmember => unreachable!("nonmember classified above"),
                }
            }
            match field.location {
                Location::Ref => {
                    changes.additions.extend(mentions);
                    if complete {
                        changes.count("ref_removed");
                        changes.removed.insert(field.name.clone());
                    } else if field.parts.is_none() {
                        changes.count("ref_not_text");
                    }
                }
                Location::Suffix => {
                    changes.replacements.insert(field.name.clone(), mentions);
                    if !field.companion {
                        changes.count("suffix_kept_no_companion");
                    } else if field.parts.is_none() {
                        changes.count("suffix_kept_not_text");
                    } else if complete && (field.empty_text || field.parts().next().is_some()) {
                        changes.count("suffix_removed");
                        changes.removed.insert(field.name.clone());
                    } else {
                        changes.count("suffix_kept_unresolved");
                    }
                }
                Location::Reviewed => {
                    changes.replacements.insert(field.name.clone(), mentions);
                    if field.parts.is_none() {
                        changes.count("reviewed_unsupported");
                    }
                }
            }
        }
        changes
    }
}

#[derive(Debug, Default, Clone)]
pub(super) struct References {
    accumulated: HashSet<String>,
    fields: HashMap<String, HashSet<String>>,
}

impl References {
    pub(super) fn apply(&mut self, changes: &mut Changes) -> Vec<String> {
        self.accumulated.extend(changes.additions.drain());
        self.fields.extend(changes.replacements.drain());
        let mentions: HashSet<_> = self
            .accumulated
            .iter()
            .chain(self.fields.values().flatten())
            .cloned()
            .collect();
        for name in &mentions {
            changes.count(if name.starts_with("r:") {
                "record_mentions"
            } else {
                "thing_mentions"
            });
        }
        mentions.into_iter().collect()
    }
}

pub(super) fn clean(payload: &mut Json, removed: &HashSet<String>) {
    if let Json::Obj(fields) = payload {
        fields.retain(|(name, _)| !removed.contains(name));
    }
}
