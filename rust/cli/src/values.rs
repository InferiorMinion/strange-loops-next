//! The values view: a thing's keyed state with each key's value read from its
//! live setters, as `loops of` shows it and `read --values` and
//! `fold --values` write it beside the ids.
//!
//! This is the CLI's reading, not the protocol's. A fold answers ids; here
//! their envelopes and held payloads are joined in, from what the store holds. Liveness
//! is the core's (`loops::live`), and nothing here compares two values.
use loops::{
    json::{Json, canon_set},
    live,
    record::{Record, Shown, id_text},
    spec::{FoldType, Follow, Slot, Spec},
};
use loops_place::dir::Held;

/// Whether a fold's answer has the values joined in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum View {
    /// The protocol's answer, as it is.
    Ids,
    /// The same answer with the values beside the ids.
    Values,
}

fn shown_name(shown: Shown) -> &'static str {
    match shown {
        Shown::Here => "here",
        Shown::Withheld => "withheld",
        Shown::Erased => "erased",
    }
}

/// The payload a reader holds of a record: none unless it is shown here.
fn payload<'a>(held: &'a Held, record: &Record) -> Option<&'a Json> {
    if record.shown != Shown::Here {
        return None;
    }
    held.line(&id_text(&record.id))?.payload()
}

/// A payload's top-level member: none when the payload lacks it or is not an object.
fn member<'a>(payload: &'a Json, key: &str) -> Option<&'a Json> {
    match payload {
        Json::Obj(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

/// One live setter of `key`: its id and `shown`, and its value of the key
/// when its payload is held and holds the key.
fn setter(held: &Held, record: &Record, key: &str) -> Json {
    let mut members = vec![
        ("id".to_owned(), Json::Str(id_text(&record.id))),
        (
            "shown".to_owned(),
            Json::Str(shown_name(record.shown).into()),
        ),
    ];
    if let Some(value) = payload(held, record).and_then(|p| member(p, key)) {
        members.push(("value".to_owned(), value.clone()));
    }
    Json::Obj(members)
}

/// Thing `thing`'s about and values over `realm`, the records held as a reader sees
/// them (settled, then admitted), as members to write beside its keyed state.
///
/// The about is the union of the mentions on its heads.
/// The keys are those some live record declares and the top-level keys of
/// each held live record read whole; a record with `sets` gives only the
/// keys it declares. Each key is read from its live setters (`live::setters`):
/// - `values` holds a key whose one live setter is held and holds the key;
///   a key whose one held setter lacks it is unset and not written;
/// - `setters` holds every other key: two or more live setters, or one whose
///   payload is not held, each with its id, its `shown` and, when held, its
///   value. Nothing picks one of two, and nothing merges them;
/// - `unheld` lists the live records read whole whose payload is not held:
///   whatever keys they set beside those written here are unknown.
///
/// `setters` and `unheld` are written only when they hold something.
pub(crate) fn thing(held: &Held, realm: &[Record], thing: &str) -> Vec<(&'static str, Json)> {
    let records = live::records_of(thing, realm);
    let keyed = live::keyed_in(thing, realm);
    let mut names: Vec<String> = keyed.keys.iter().map(|(k, _)| k.clone()).collect();
    for record in &keyed.whole {
        if let Some(Json::Obj(members)) = payload(held, record) {
            names.extend(members.iter().map(|(k, _)| k.clone()));
        }
    }
    let mut values = Vec::new();
    let mut several = Vec::new();
    for key in canon_set(&names) {
        let live = live::setters(&records, &key);
        match live.as_slice() {
            [] => {}
            [one] if one.shown == Shown::Here => {
                if let Some(value) = payload(held, one).and_then(|p| member(p, &key)) {
                    values.push((key, value.clone()));
                }
            }
            _ => {
                let each = live.iter().map(|r| setter(held, r, &key)).collect();
                several.push((key, Json::Arr(each)));
            }
        }
    }
    let unheld: Vec<Json> = keyed
        .whole
        .iter()
        .filter(|r| r.shown != Shown::Here)
        .map(|r| {
            Json::Obj(vec![
                ("id".to_owned(), Json::Str(id_text(&r.id))),
                ("shown".to_owned(), Json::Str(shown_name(r.shown).into())),
            ])
        })
        .collect();
    let mentions: Vec<_> = keyed
        .heads
        .iter()
        .flat_map(|r| r.about.iter().cloned())
        .collect();
    let about = Json::Arr(canon_set(&mentions).into_iter().map(Json::Str).collect());
    let mut out = vec![("about", about), ("values", Json::Obj(values))];
    if !several.is_empty() {
        out.push(("setters", Json::Obj(several)));
    }
    if !unheld.is_empty() {
        out.push(("unheld", Json::Arr(unheld)));
    }
    out
}

/// Applies `f` to the member `name` of an object, leaving everything else.
fn at_member(value: Json, name: &str, f: impl Fn(Json) -> Json) -> Json {
    match value {
        Json::Obj(members) => Json::Obj(
            members
                .into_iter()
                .map(|(k, v)| if k == name { (k, f(v)) } else { (k, v) })
                .collect(),
        ),
        other => other,
    }
}

/// Applies `f` to each member of an object, with its name.
fn each_member(value: Json, f: impl Fn(&str, Json) -> Json) -> Json {
    match value {
        Json::Obj(members) => Json::Obj(
            members
                .into_iter()
                .map(|(k, v)| {
                    let v = f(&k, v);
                    (k, v)
                })
                .collect(),
        ),
        other => other,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryView {
    Envelope,
    EnvelopeAndPayload,
}

/// Add the stored envelope to each entry, and its held payload when requested.
fn entries(held: &Held, list: Json, view: EntryView) -> Json {
    let Json::Arr(mut entries) = list else {
        return list;
    };
    for entry in &mut entries {
        let Some(Json::Str(id)) = member(entry, "id") else {
            continue;
        };
        let Some(line) = held.line(id) else {
            continue;
        };
        let here = matches!(member(entry, "shown"), Some(Json::Str(s)) if s == "here");
        if let Json::Obj(members) = entry {
            members.push(("env".to_owned(), line.env().to_json()));
            if view == EntryView::EnvelopeAndPayload && here {
                if let Some(p) = line.payload() {
                    members.push(("payload".to_owned(), p.clone()));
                }
            }
        }
    }
    Json::Arr(entries)
}

/// Add each thing's about and values, and the envelopes of its heads.
/// A collect or latest-by following `prev` also shows held payloads.
fn slot(held: &Held, realm: &[Record], slot: &Slot, value: Json) -> Json {
    let records = |list| entries(held, list, EntryView::EnvelopeAndPayload);
    if slot.group.is_some() {
        return at_member(value, "groups", |groups| {
            each_member(groups, |_, group| at_member(group, "heads", records))
        });
    }
    match slot.fold_type {
        FoldType::LatestBy {
            follow: Follow::Keys,
        } => at_member(value, "things", |things| {
            each_member(things, |name, view| {
                match at_member(view, "heads", |list| {
                    entries(held, list, EntryView::Envelope)
                }) {
                    Json::Obj(mut members) => {
                        members.extend(
                            thing(held, realm, name)
                                .into_iter()
                                .map(|(k, v)| (k.to_owned(), v)),
                        );
                        Json::Obj(members)
                    }
                    other => other,
                }
            })
        }),
        FoldType::LatestBy {
            follow: Follow::Prev,
        } => at_member(value, "things", |things| {
            each_member(things, |_, list| records(list))
        }),
        FoldType::Collect { .. } => at_member(value, "heads", records),
        FoldType::Count { .. }
        | FoldType::Sum { .. }
        | FoldType::Min { .. }
        | FoldType::Max { .. } => value,
    }
}

/// A fold's answer, `slots`, with the values joined in, slot by slot.
pub(crate) fn fold(held: &Held, realm: &[Record], spec: &Spec, slots: Json) -> Json {
    each_member(slots, |name, value| {
        match spec.slots.iter().find(|s| s.name == name) {
            Some(s) => slot(held, realm, s, value),
            None => value,
        }
    })
}
