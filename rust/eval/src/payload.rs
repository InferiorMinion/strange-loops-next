//! The fixture representation of a projected payload.
use crate::reader::{members, required};
use loops::{
    json::Json,
    record::{Held, Val, payload_fields},
};

fn tag(key: &str, value: Json) -> Json {
    Json::Obj(vec![(key.into(), value)])
}

fn scalar(v: Val) -> Json {
    match v {
        Val::Str(s) => tag("text", Json::Str(s)),
        Val::Int(i) => tag("whole", Json::Str(i.to_string())),
        Val::Number(n) => tag("number", Json::Str(n.canon())),
        Val::Bool(b) => tag("bool", Json::Bool(b)),
    }
}

/// Encodes already-projected reader fields in the payload vector format.
pub fn fixture_fields(fields: Vec<(String, Held)>) -> Json {
    Json::Obj(
        fields
            .into_iter()
            .map(|(k, h)| {
                let value = match h {
                    Held::One(v) => tag("one", scalar(v)),
                    Held::Many(vs) => tag("many", Json::Arr(vs.into_iter().map(scalar).collect())),
                    Held::Other => tag("other", Json::Bool(true)),
                };
                (k, value)
            })
            .collect(),
    )
}

pub(crate) fn answers(layer: &str, input: &Json) -> Result<Vec<Json>, String> {
    if layer != "payload" {
        return Err(format!("unknown layer {layer}"));
    }
    let kvs = members("input", &["payload"], input)?;
    let p = required("input", kvs, "payload")?;
    Ok(vec![fields(p)?])
}

pub(super) fn fields(p: &Json) -> Result<Json, String> {
    if !p.keys_distinct() || !p.in_range() {
        return Err("input.payload: invalid stored payload".into());
    }
    Ok(tag("fields", fixture_fields(payload_fields(p))))
}
