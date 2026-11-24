//! Inverse reference lookup over the admitted union, using the keyed fold's payload test.
use crate::{
    Fail, Format, count, finish_answer, object, plain, realm, references::count_values,
    registry::SelectedStore, say, selection::Union, text, usage, values::setter_values,
};
use loops::{
    envelope::{record_id, typed_ref},
    families::Tri,
    json::{Json, str_cmp},
    live::LiveState,
    record::{Record, Val},
    where_::{Arg, Cond, Field, Op},
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    path::Path,
};

// --fields takes a literal operand, even if that operand looks like a flag.
pub(crate) fn target(args: &[String]) -> Result<Option<&str>, Fail> {
    let mut operands = args.iter();
    while let Some(arg) = operands.next() {
        match arg.as_str() {
            "--fields" => {
                operands.next();
            }
            "--refers-to" => {
                if let [flag, name] = args {
                    if flag == "--refers-to"
                        && typed_ref(name)
                        && (!name.starts_with("r:") || record_id(name))
                    {
                        return Ok(Some(name));
                    }
                }
                return Err(usage(
                    "--refers-to takes one complete typed target and no other ls operands",
                ));
            }
            _ => {}
        }
    }
    Ok(None)
}

struct Declaration<'a> {
    kind: &'a str,
    field: &'a str,
    descriptions: Vec<String>,
}

fn declarations(union: &Union) -> Vec<Declaration<'_>> {
    let mut fields = HashMap::<_, Vec<String>>::new();
    for site in &union.sites {
        let Some((desc, _)) = &site.description else {
            continue;
        };
        let path = Path::new(&site.directory).join(".loops/store.json");
        let path = path.to_str().expect("UTF-8 store directory");
        for (kind, rule) in &desc.kinds {
            for field in rule.refs.iter().flat_map(|refs| refs.keys()) {
                fields
                    .entry((kind.as_str(), field.as_str()))
                    .or_default()
                    .push(path.to_owned());
            }
        }
    }
    let mut rows: Vec<_> = fields
        .into_iter()
        .map(|((kind, field), mut descriptions)| {
            descriptions.sort_unstable_by(|a, b| str_cmp(a, b));
            Declaration {
                kind,
                field,
                descriptions,
            }
        })
        .collect();
    rows.sort_unstable_by(|a, b| str_cmp(a.kind, b.kind).then_with(|| str_cmp(a.field, b.field)));
    rows
}

#[derive(Default)]
struct Matches {
    things: Vec<String>,
    things_not_read: usize,
    values_not_typed: usize,
}

fn matches(union: &Union, fields: &[Declaration<'_>], target: &str) -> Matches {
    let mut by_kind = HashMap::<_, Vec<_>>::new();
    let mut tests = HashMap::new();
    for row in fields {
        by_kind.entry(row.kind).or_default().push(row.field);
        tests.entry(row.field).or_insert_with(|| Cond {
            field: Field::Payload(row.field.to_owned()),
            op: Op::Is(Arg::Val(Val::Str(target.to_owned()))),
        });
    }
    let mut groups = HashMap::<_, Vec<Record>>::new();
    for record in realm(&union.held) {
        groups.entry(record.thing()).or_default().push(record);
    }
    let mut result = Matches::default();
    for (name, records) in groups {
        let state = LiveState::new(records.iter().collect());
        let fields: HashSet<_> = state
            .heads()
            .iter()
            .filter_map(|head| by_kind.get(head.kind.as_str()))
            .flatten()
            .copied()
            .collect();
        let mut answer = Tri::No;
        for field in fields {
            // Coverage includes all applicable fields, even after a yes.
            result.values_not_typed +=
                count_values(setter_values(&union.held, &state, field), |_| {}).not_typed;
            answer = match (answer, tests[field].on_thing(&records)) {
                (Tri::Yes, _) | (_, Tri::Yes) => Tri::Yes,
                (Tri::Unknown, _) | (_, Tri::Unknown) => Tri::Unknown,
                _ => Tri::No,
            };
        }
        match answer {
            Tri::Yes => result.things.push(name),
            Tri::Unknown => result.things_not_read += 1,
            Tri::No => {}
        }
    }
    result.things.sort_unstable_by(|a, b| str_cmp(a, b));
    result
}

fn strings(items: &[String]) -> Json {
    Json::Arr(items.iter().map(text).collect())
}

pub(crate) fn run(stores: Vec<SelectedStore>, target: &str, format: Format) -> Result<(), Fail> {
    let mut union = Union::read(stores)?;
    union
        .sites
        .sort_unstable_by(|a, b| str_cmp(&a.directory, &b.directory));
    let fields = declarations(&union);
    let result = matches(&union, &fields, target);
    let counts = [
        ("things_not_read", result.things_not_read),
        ("values_not_typed", result.values_not_typed),
        ("fields_read", fields.len()),
    ];
    let output = match format {
        Format::Json => {
            let rows = fields.iter().map(|row| {
                object(vec![
                    ("kind", text(row.kind)),
                    ("field", text(row.field)),
                    ("descriptions", strings(&row.descriptions)),
                ])
            });
            object(vec![
                ("target", text(target)),
                ("things", strings(&result.things)),
                (
                    "stores",
                    Json::Arr(union.sites.iter().map(|s| text(&s.directory)).collect()),
                ),
                ("fields", Json::Arr(rows.collect())),
                (
                    "counts",
                    object(counts.map(|(key, n)| (key, count(n))).into()),
                ),
            ])
            .canon()
        }
        Format::Plain => {
            let mut out = format!(
                "target  {}\nthings  {}",
                plain::label(target),
                result.things.len()
            );
            for name in &result.things {
                write!(out, "\n  {}", plain::label(name)).expect("write string");
            }
            write!(out, "\nstores  {}", union.sites.len()).expect("write string");
            for site in &union.sites {
                write!(out, "\n  {}", plain::label(&site.directory)).expect("write string");
            }
            let table = plain::table(
                vec!["KIND".into(), "FIELD".into(), "DESCRIPTION".into()],
                fields.iter().flat_map(|row| {
                    row.descriptions.iter().map(|path| {
                        vec![
                            plain::label(row.kind),
                            plain::label(row.field),
                            plain::label(path),
                        ]
                    })
                }),
            );
            write!(out, "\n{table}").expect("write string");
            for (label, n) in counts {
                write!(out, "\n{label}  {n}").expect("write string");
            }
            out
        }
    };
    finish_answer(say(&output), union.integrity)
}
