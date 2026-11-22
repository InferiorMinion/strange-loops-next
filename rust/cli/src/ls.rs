//! Inventory of the settled, admitted records in a store.
use crate::{Fail, answered, count, find_store, object, realm, say, text, usage};
use loops::{
    json::{Json, canon_set, str_cmp},
    live::LiveState,
    record::{Held, Record, Shown, Val},
};
use std::{collections::HashMap, path::Path};

enum Headline {
    Text(String),
    Absent,
    Unknown,
    Conflict,
}

impl Headline {
    fn from_state(state: &LiveState<'_>) -> Self {
        let setters = state.setters("name");
        match setters.as_slice() {
            [one] if one.shown == Shown::Here => match one.get("name") {
                Some(Held::One(Val::Str(value))) => Self::Text(value),
                _ => Self::Absent,
            },
            // Whole records implicitly set every key. If none even mentions
            // name, report absence (or uncertainty if an opening is missing).
            many if many.len() > 1
                && many.iter().any(|r| {
                    r.declares().iter().any(|k| k == "name")
                        || (r.shown == Shown::Here && r.fields.iter().any(|(k, _)| k == "name"))
                }) =>
            {
                Self::Conflict
            }
            some if some.iter().any(|r| r.shown != Shown::Here) => Self::Unknown,
            _ => Self::Absent,
        }
    }

    fn into_json(self) -> Json {
        let state = match self {
            Self::Text(value) => {
                return object(vec![("state", text("text")), ("value", text(value))]);
            }
            Self::Absent => "absent",
            Self::Unknown => "unknown",
            Self::Conflict => "conflict",
        };
        object(vec![("state", text(state))])
    }
}

struct ThingRow {
    thing: String,
    kinds: Vec<String>,
    records: usize,
    heads: usize,
    headline: Headline,
}

impl ThingRow {
    fn new(thing: String, records: Vec<&Record>) -> Self {
        let n = records.len();
        let state = LiveState::new(records);
        let heads = state.heads();
        let kinds = heads.iter().map(|r| r.kind.clone()).collect::<Vec<_>>();
        Self {
            thing,
            kinds: canon_set(&kinds),
            records: n,
            heads: heads.len(),
            headline: Headline::from_state(&state),
        }
    }

    fn into_json(self) -> Json {
        object(vec![
            ("thing", text(self.thing)),
            (
                "kinds",
                Json::Arr(self.kinds.into_iter().map(text).collect()),
            ),
            ("records", count(self.records)),
            ("heads", count(self.heads)),
            ("headline", self.headline.into_json()),
        ])
    }
}

/// The same UTF-16 order used for names throughout the core.
fn sorted<T>(groups: HashMap<String, T>) -> Vec<(String, T)> {
    let mut rows: Vec<_> = groups.into_iter().collect();
    rows.sort_unstable_by(|(a, _), (b, _)| str_cmp(a, b));
    rows
}

fn kind_counts(records: &[Record]) -> Vec<(String, usize)> {
    let mut counts = HashMap::new();
    for record in records {
        *counts.entry(record.kind.clone()).or_default() += 1;
    }
    sorted(counts)
}

fn thing_rows(records: &[Record], prefix: &str) -> Vec<ThingRow> {
    let mut groups = HashMap::<_, Vec<_>>::new();
    for record in records {
        let thing = record.thing();
        if thing.starts_with(prefix) {
            groups.entry(thing).or_default().push(record);
        }
    }
    sorted(groups)
        .into_iter()
        .map(|(name, rs)| ThingRow::new(name, rs))
        .collect()
}

pub(crate) fn run(start: &Path, args: &[String]) -> Result<(), Fail> {
    let prefix = match args {
        [] => None,
        [arg] if arg == "--things" => Some(""),
        [arg] if !arg.starts_with("--") => Some(arg.as_str()),
        _ => return Err(usage("ls takes --things, one name prefix, or no arguments")),
    };
    let held = find_store(start)?.read()?;
    let records = realm(&held);
    let (key, rows) = match prefix {
        None => (
            "kinds",
            kind_counts(&records)
                .into_iter()
                .map(|(kind, n)| object(vec![("kind", text(kind)), ("records", count(n))]))
                .collect(),
        ),
        Some(p) => (
            "things",
            thing_rows(&records, p)
                .into_iter()
                .map(ThingRow::into_json)
                .collect(),
        ),
    };
    answered(&held, say(&object(vec![(key, Json::Arr(rows))]).canon()))
}
