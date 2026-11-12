//! Gen.SpecVectors parsing, execution, and slot rendering from model/Gen/Spec.lean.
use super::{
    json::parse_overflowing,
    reader::{list, members, record, required},
};
use loops::{
    json::Json,
    record::{Digest, Shown, id_text},
    spec::{GroupValue, SlotValue, Spec},
};

fn integer(i: impl std::fmt::Display) -> Json {
    Json::Int(i.to_string().parse().expect("unbounded integer result"))
}
fn object(kvs: Vec<(&str, Json)>) -> Json {
    Json::Obj(kvs.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
pub(super) fn shown_ids(hs: Vec<(Digest, Shown)>) -> Json {
    Json::Arr(
        hs.into_iter()
            .map(|(id, shown)| {
                object(vec![
                    ("id", Json::Str(id_text(&id))),
                    (
                        "shown",
                        Json::Str(
                            match shown {
                                Shown::Here => "here",
                                Shown::Withheld => "withheld",
                                Shown::Erased => "erased",
                            }
                            .into(),
                        ),
                    ),
                ])
            })
            .collect(),
    )
}
/// What one group holds, written as its slot without `by` is.
fn group(v: GroupValue) -> Json {
    let (key, value, unread) = match v {
        GroupValue::Nat(r) => ("value", integer(r.value), r.unread),
        GroupValue::Int(r) => ("value", Json::Int(r.value), r.unread),
        GroupValue::Opt(r) => ("value", r.value.map_or(Json::Null, Json::Int), r.unread),
        GroupValue::Heads { heads, unread } => ("heads", shown_ids(heads), unread),
    };
    object(vec![(key, value), ("unread", integer(unread))])
}
pub(super) fn slot(v: SlotValue) -> Json {
    match v {
        SlotValue::Nat(r) => group(GroupValue::Nat(r)),
        SlotValue::Int(r) => group(GroupValue::Int(r)),
        SlotValue::Opt(r) => group(GroupValue::Opt(r)),
        SlotValue::Heads { heads, unread } => group(GroupValue::Heads { heads, unread }),
        SlotValue::Things { things, unread } => object(vec![
            (
                "things",
                Json::Obj(
                    things
                        .into_iter()
                        .map(|(t, hs)| (t, shown_ids(hs)))
                        .collect(),
                ),
            ),
            ("unread", integer(unread)),
        ]),
        SlotValue::Groups {
            groups,
            unread,
            ungrouped,
        } => object(vec![
            (
                "groups",
                Json::Obj(groups.into_iter().map(|(k, v)| (k, group(v))).collect()),
            ),
            ("unread", integer(unread)),
            ("ungrouped", integer(ungrouped)),
        ]),
    }
}
pub(super) fn parsed(j: &Json) -> Result<Spec, String> {
    let sp = Spec::of_json(j).map_err(|e| e.to_string())?;
    let back = Spec::of_json(&sp.to_json()).map_err(|e| format!("round trip refused: {e}"))?;
    if back != sp {
        return Err("parse-toJson round trip changes spec".into());
    }
    Ok(sp)
}
/// Gen.SpecVectors.parseVector/runVector, model/Gen/Spec.lean.
/// Checks the closed dispatch and retains raw spec text before parsing.
pub(crate) fn answers(layer: &str, input: &Json) -> Result<Vec<Json>, String> {
    match layer {
        "spec/parse" => {
            let kvs = members("input", &["spec", "spec_text"], input)?;
            let raw = kvs.iter().find(|(k, _)| k == "spec_text").map(|(_, j)| j);
            let written = match raw {
                Some(Json::Str(text)) => {
                    Some(parse_overflowing(text).map_err(|e| format!("input.spec_text: {e}"))?)
                }
                None => None,
                Some(_) => return Err("input.spec_text: expected string".into()),
            };
            let j = match &written {
                Some(j) => j,
                None => required("input", kvs, "spec")?,
            };
            let answer = match Spec::of_json(j) {
                Ok(_) => object(vec![("parsed", parsed(j)?.to_json())]),
                Err(e) => object(vec![
                    ("refused", Json::Str(e.code().into())),
                    ("path", Json::Str(e.path)),
                ]),
            };
            Ok(vec![answer])
        }
        "spec/run" => {
            let kvs = members("input", &["spec", "orders"], input)?;
            let sp = parsed(required("input", kvs, "spec")?)?;
            let orders = list("input.orders", required("input", kvs, "orders")?)?;
            if orders.is_empty() {
                return Err("input.orders: no arrival order".into());
            }
            orders
                .iter()
                .enumerate()
                .map(|(i, o)| {
                    let path = format!("input.orders[{i}]");
                    let xs = list(&path, o)?
                        .iter()
                        .enumerate()
                        .map(|(k, r)| record(&format!("{path}[{k}]"), r))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(Json::Obj(
                        sp.run(&xs).into_iter().map(|(n, v)| (n, slot(v))).collect(),
                    ))
                })
                .collect()
        }
        _ => Err(format!("unknown spec layer {layer}")),
    }
}

#[cfg(test)]
mod tests {
    use super::answers;
    use crate::parse;
    #[test]
    fn closed_dispatch_and_strict_input() {
        for (layer, raw) in [
            ("spec/future", "{}"),
            (
                "spec/run",
                r#"{"spec":{"vertex":"x","from":[{"store":"."}]},"orders":[]}"#,
            ),
            (
                "spec/run",
                r#"{"spec":{"vertex":"x","from":[{"store":"."}]},"orders":[[{}]]}"#,
            ),
            ("spec/parse", r#"{"spec":{},"spec_text":3}"#),
            ("spec/parse", r#"{"spec":{},"extra":1}"#),
            ("spec/parse", "{}"),
        ] {
            assert!(answers(layer, &parse(raw)).is_err(), "{layer} {raw}");
        }
    }
}
