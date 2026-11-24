//! Inventory of the settled, admitted records in a store.
use crate::description::{Description, Lifecycle, prefix_kind};
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
    fn from_records(
        held: &Held,
        thing: String,
        records: Vec<&Record>,
        names: &[&str],
        desc: Option<&Description>,
        lifecycle: Option<&Lifecycle>,
    ) -> Option<Self> {
        let n = records.len();
        let state = LiveState::new(records);
        let reader = Fields::new(held, &state);
        if let Some(rule) = lifecycle {
            if matches!(reader.read(&rule.field), Field::Value(Json::Str(s)) if rule.closed().contains(&s))
            {
                return None;
            }
        }
        let heads = state.heads();
        let kinds = heads.iter().map(|r| r.kind.clone()).collect::<Vec<_>>();
        Some(Self {
            thing,
            kinds: canon_set(&kinds),
            records: n,
            heads: heads.len(),
            headline: reader.read(desc.and_then(|d| d.headline(&heads)).unwrap_or("name")),
            fields: names.iter().map(|k| reader.read(k)).collect(),
        })
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
    lifecycle: Option<&Lifecycle>,
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
        .filter_map(|(name, rs)| ThingRow::from_records(held, name, rs, names, desc, lifecycle))
        .collect()
}

const LS_OPERANDS: &str = "ls takes --things, one name prefix, or no arguments";
const ACTIVE_PREFIX: &str = "--active needs a <kind>: prefix whose kind declares lifecycle";

struct Listing<'a> {
    prefix: Option<&'a str>,
    names: Vec<&'a str>,
    active_kind: Option<&'a str>,
}

fn listing(args: &[String]) -> Result<Listing<'_>, Fail> {
    let mut active = false;
    let mut selectors = Vec::new();
    let mut names = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--active" if active => return Err(usage("use --active once")),
            "--active" => active = true,
            "--fields" if names.is_empty() => {
                // Keep an earlier extra-operand error ahead of a malformed field list.
                let invalid = |message| {
                    usage(if selectors.len() > 1 {
                        LS_OPERANDS
                    } else {
                        message
                    })
                };
                let list = args
                    .next()
                    .ok_or_else(|| invalid("--fields takes comma-separated field names"))?;
                for key in list.split(',') {
                    if key.is_empty() || names.contains(&key) {
                        return Err(invalid("--fields needs distinct, nonempty field names"));
                    }
                    names.push(key);
                }
            }
            "--things" => selectors.push(""),
            p if !p.starts_with("--") => selectors.push(p),
            _ => return Err(usage(LS_OPERANDS)),
        }
    }
    let prefix = match selectors.as_slice() {
        [] => None,
        &[p] => Some(p),
        _ if active => return Err(usage(ACTIVE_PREFIX)),
        _ => return Err(usage(LS_OPERANDS)),
    };
    let active_kind = if active {
        Some(
            prefix
                .and_then(prefix_kind)
                .ok_or_else(|| usage(ACTIVE_PREFIX))?,
        )
    } else {
        None
    };
    Ok(Listing {
        prefix,
        names,
        active_kind,
    })
}

pub(crate) fn run(ctx: &Context<'_>, args: &[String], format: plain::Format) -> Result<(), Fail> {
    let Listing {
        prefix,
        mut names,
        active_kind,
    } = listing(args)?;
    let store = ctx.store()?;
    let desc = if format == plain::Format::Plain || active_kind.is_some() {
        Description::load(store.dir.root())?
    } else {
        None
    };
    let active = if let Some(kind) = active_kind {
        let rule = desc
            .as_ref()
            .and_then(|d| d.lifecycle(kind))
            .ok_or_else(|| usage(ACTIVE_PREFIX))?;
        Some((kind, rule))
    } else {
        None
    };
    // Headline and preview defaults belong only to the plain rendering.
    let plain_desc = desc.as_ref().filter(|_| format == plain::Format::Plain);
    let held = store.read()?;
    let records = realm(&held);
    let output = if let Some(p) = prefix.or_else(|| (!names.is_empty()).then_some("")) {
        if names.is_empty() {
            if let Some(preview) = plain_desc.and_then(|d| d.preview(p)) {
                names.extend(preview.iter().map(String::as_str));
            }
        }
        let rows = thing_rows(
            &held,
            &records,
            p,
            &names,
            plain_desc,
            active
                .map(|(_, rule)| rule)
                .filter(|rule| !rule.closed().is_empty()),
        );
        let applied = active.map(|(kind, rule)| {
            object(vec![
                ("kind", text(kind)),
                ("field", text(&rule.field)),
                (
                    "closed",
                    Json::Arr(rule.closed().iter().map(text).collect()),
                ),
            ])
        });
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
            let table = plain::table(header, rows.iter().map(|row| row.cells(&ids)));
            match applied {
                Some(rule) => format!("active  {}\n{table}", plain::value(&rule)),
                None => table,
            }
        } else {
            let mut members = vec![(
                "things",
                Json::Arr(rows.into_iter().map(|row| row.into_json(&names)).collect()),
            )];
            if let Some(rule) = applied {
                members.push(("active", rule));
            }
            object(members).canon()
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
