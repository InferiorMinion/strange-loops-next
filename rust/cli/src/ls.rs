//! Inventory of the settled, admitted records in a store.
use crate::description::Description;
use crate::values::{Field, Fields};
use crate::{Context, Fail, answered, count, object, plain, realm, say, text, usage};
use loops::{
    json::{Json, canon_set, str_cmp},
    live::LiveState,
    record::Record,
};
use loops_place::dir::Held;
use std::collections::HashMap;

struct ThingRow {
    thing: String,
    kinds: Vec<String>,
    records: usize,
    heads: usize,
    headline: Field,
    fields: Vec<Field>,
}

impl ThingRow {
    fn new(
        held: &Held,
        thing: String,
        records: Vec<&Record>,
        names: &[&str],
        desc: Option<&Description>,
    ) -> Self {
        let n = records.len();
        let state = LiveState::new(records);
        let heads = state.heads();
        let kinds = heads.iter().map(|r| r.kind.clone()).collect::<Vec<_>>();
        let reader = Fields::new(held, &state);
        Self {
            thing,
            kinds: canon_set(&kinds),
            records: n,
            heads: heads.len(),
            headline: reader.read(desc.and_then(|d| d.headline(&heads)).unwrap_or("name")),
            fields: names.iter().map(|k| reader.read(k)).collect(),
        }
    }

    fn cells(&self, ids: &plain::Prefixes<'_>) -> Vec<String> {
        let mut cells = vec![
            plain::label(ids.short(&self.thing)),
            self.kinds
                .iter()
                .map(|k| plain::label(k))
                .collect::<Vec<_>>()
                .join(", "),
            self.records.to_string(),
            self.heads.to_string(),
        ];
        if self.fields.is_empty() {
            cells.push(plain::headline(&self.headline));
        } else {
            cells.extend(self.fields.iter().map(plain::field));
        }
        cells
    }

    fn into_json(self, names: &[&str]) -> Json {
        let headline = match self.headline {
            Field::Value(Json::Str(s)) => object(vec![("state", text("text")), ("value", text(s))]),
            Field::Value(_) | Field::Unset => object(vec![("state", text("absent"))]),
            Field::Unknown(_) => object(vec![("state", text("unknown"))]),
            Field::Conflict(_) => object(vec![("state", text("conflict"))]),
        };
        let mut members = vec![
            ("thing", text(self.thing)),
            (
                "kinds",
                Json::Arr(self.kinds.into_iter().map(text).collect()),
            ),
            ("records", count(self.records)),
            ("heads", count(self.heads)),
            ("headline", headline),
        ];
        if !names.is_empty() {
            members.push((
                "fields",
                Json::Obj(
                    names
                        .iter()
                        .zip(self.fields)
                        .map(|(k, v)| ((*k).to_owned(), v.projection()))
                        .collect(),
                ),
            ));
        }
        object(members)
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

fn thing_rows(
    held: &Held,
    records: &[Record],
    prefix: &str,
    names: &[&str],
    desc: Option<&Description>,
) -> Vec<ThingRow> {
    let mut groups = HashMap::<_, Vec<_>>::new();
    for record in records {
        let thing = record.thing();
        if thing.starts_with(prefix) {
            groups.entry(thing).or_default().push(record);
        }
    }
    sorted(groups)
        .into_iter()
        .map(|(name, rs)| ThingRow::new(held, name, rs, names, desc))
        .collect()
}

pub(crate) fn run(ctx: &Context<'_>, args: &[String], format: plain::Format) -> Result<(), Fail> {
    let mut prefix = None;
    let mut names = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fields" if names.is_empty() => {
                let list = args
                    .next()
                    .ok_or_else(|| usage("--fields takes comma-separated field names"))?;
                for key in list.split(',') {
                    if key.is_empty() || names.contains(&key) {
                        return Err(usage("--fields needs distinct, nonempty field names"));
                    }
                    names.push(key);
                }
            }
            "--things" if prefix.is_none() => prefix = Some(""),
            p if !p.starts_with("--") && prefix.is_none() => prefix = Some(p),
            _ => {
                return Err(usage("ls takes --things, one name prefix, or no arguments"));
            }
        }
    }
    let store = ctx.store()?;
    let desc = match format {
        plain::Format::Plain => Description::load(store.dir.root())?,
        plain::Format::Json => None,
    };
    let held = store.read()?;
    let records = realm(&held);
    let output = if let Some(p) = prefix.or_else(|| (!names.is_empty()).then_some("")) {
        if names.is_empty() {
            if let Some(preview) = desc.as_ref().and_then(|d| d.preview(p)) {
                names.extend(preview.iter().map(String::as_str));
            }
        }
        let rows = thing_rows(&held, &records, p, &names, desc.as_ref());
        if format == plain::Format::Plain {
            let mut header = vec![
                "THING".into(),
                "HEAD KINDS".into(),
                "RECORDS".into(),
                "HEADS".into(),
            ];
            if names.is_empty() {
                header.push("HEADLINE".into());
            } else {
                header.extend(names.iter().map(|k| plain::label(k)));
            }
            let ids = plain::Prefixes::new(&held);
            plain::table(header, rows.iter().map(|row| row.cells(&ids)))
        } else {
            object(vec![(
                "things",
                Json::Arr(rows.into_iter().map(|row| row.into_json(&names)).collect()),
            )])
            .canon()
        }
    } else {
        let rows = kind_counts(&records);
        if format == plain::Format::Plain {
            plain::table(
                vec!["KIND".into(), "RECORDS".into()],
                rows.iter()
                    .map(|(k, n)| vec![plain::label(k), n.to_string()]),
            )
        } else {
            object(vec![(
                "kinds",
                Json::Arr(
                    rows.into_iter()
                        .map(|(k, n)| object(vec![("kind", text(k)), ("records", count(n))]))
                        .collect(),
                ),
            )])
            .canon()
        }
    };
    answered(&held, say(&output))
}
