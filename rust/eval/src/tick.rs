//! Input-only answers for the tick vector family.
pub(crate) mod check;
use crate::reader::{list, members, object, record, required, text};
use loops::{
    json::Json,
    record::{Record, Shown, payload_fields},
    result::{result_json, result_preimage},
    spec::Spec,
    spec_hash::{Rules, spec_hash_of, spec_label, spec_preimage_of},
    tick::{TickReadError, make_tick, read_tick_members, read_tick_payload, tick_ids},
    where_::Refusal,
};
use sha2::{Digest, Sha256};

fn sha256(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

fn read_rules(j: &Json) -> Result<Rules, String> {
    Rules::of_json(j).ok_or_else(|| "rules: expected canonical positive decimal text".into())
}

fn hash(input: &Json) -> Result<Json, String> {
    let kvs = members("input", &["op", "samples"], input)?;
    let samples = list("input.samples", required("input", kvs, "samples")?)?;
    samples
        .iter()
        .enumerate()
        .map(|(i, sample)| {
            let path = format!("input.samples[{i}]");
            let kvs = members(&path, &["rules", "spec"], sample)?;
            let rules = read_rules(required(&path, kvs, "rules")?)?;
            let raw = required(&path, kvs, "spec")?;
            let raw_hash = spec_hash_of(&sha256, &rules, raw);
            let spec = crate::spec::parsed(raw)?;
            Ok(Json::Obj(vec![
                ("parsed-jcs".into(), Json::Str(spec.to_json().canon())),
                ("spec-hash".into(), Json::Str(spec.hash(&sha256, &rules))),
                ("raw-member-hash".into(), Json::Str(raw_hash)),
            ]))
        })
        .collect::<Result<_, _>>()
        .map(Json::Arr)
}

fn domains(input: &Json) -> Result<Json, String> {
    let kvs = members("input", &["op", "rules", "spec"], input)?;
    let rules = read_rules(required("input", kvs, "rules")?)?;
    let spec = crate::spec::parsed(required("input", kvs, "spec")?)?;
    let label = spec_label(&rules);
    let bytes = spec_preimage_of(&rules, &spec.to_json());
    let suffix = std::str::from_utf8(&bytes[16..]).map_err(|e| e.to_string())?;
    let byte = |b: u8| Json::Int(u64::from(b).into());
    let label_bytes = label.bytes().map(byte).collect();
    Ok(Json::Obj(vec![
        ("label".into(), Json::Str(label)),
        ("label-bytes".into(), Json::Arr(label_bytes)),
        ("after-salt".into(), Json::Str(suffix.into())),
        (
            "after-salt-is-json".into(),
            Json::Bool(crate::parse_json(suffix).is_ok()),
        ),
        ("spec-byte-7".into(), byte(bytes[6])),
        ("result-byte-7".into(), byte(result_preimage(&[])[6])),
    ]))
}

fn spec_records(input: &Json) -> Result<(Spec, Vec<Record>), String> {
    let kvs = members("input", &["op", "spec", "records"], input)?;
    let spec = crate::spec::parsed(required("input", kvs, "spec")?)?;
    let records = list("input.records", required("input", kvs, "records")?)?
        .iter()
        .enumerate()
        .map(|(i, r)| record(&format!("input.records[{i}]"), r))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((spec, records))
}

fn window(input: &Json) -> Result<Json, String> {
    let (spec, records) = spec_records(input)?;
    Ok(result_json(&spec.run(&records)))
}

fn refusal(f: Refusal) -> Json {
    Json::Obj(vec![
        ("refused".into(), Json::Str(f.code().into())),
        ("path".into(), Json::Str(f.path)),
    ])
}

fn read_back(raw: &Json) -> Result<Json, String> {
    let current = Rules::current();
    let payload = match read_tick_payload(&sha256, &current, raw) {
        Ok(p) => p,
        Err(TickReadError::Invalid(f)) => return Ok(refusal(f)),
        Err(TickReadError::OtherRules(v)) => {
            return Ok(Json::Obj(vec![("other-rules".into(), Json::Str(v.text()))]));
        }
        Err(TickReadError::UnreadableSpec) => {
            return Ok(Json::Obj(vec![(
                "cannot-read".into(),
                Json::Str("spec".into()),
            )]));
        }
    };
    let encoded = payload.to_json(&sha256);
    let jcs = encoded.canon();
    if jcs != raw.canon() {
        return Err("reading changed the payload's canonical bytes".into());
    }
    let canonical = crate::parse_json(&jcs).map_err(|e| e.to_string())?;
    let again =
        read_tick_payload(&sha256, &current, &canonical).map_err(|_| "JCS payload did not read")?;
    if again.bytes(&sha256) != jcs.as_bytes() {
        return Err("second reading changed the bytes".into());
    }
    Ok(Json::Obj(vec![
        ("payload".into(), encoded),
        ("jcs".into(), Json::Str(jcs)),
    ]))
}

fn write_payload(input: &Json) -> Result<Json, String> {
    let (spec, records) = spec_records(input)?;
    let payload = make_tick(spec, &records);
    let j = payload.to_json(&sha256);
    let q =
        read_tick_payload(&sha256, &payload.rules, &j).map_err(|_| "written payload refused")?;
    // Compare the result with shown erased, retaining all member and list orders.
    if q.spec != payload.spec
        || q.covers != payload.covers
        || q.withheld != payload.withheld
        || result_json(&q.result) != result_json(&payload.result)
    {
        return Err("written result did not read back in normal form".into());
    }
    read_back(&j)
}

fn payload_member(input: &Json) -> Result<&Json, String> {
    required(
        "input",
        members("input", &["op", "payload"], input)?,
        "payload",
    )
}

fn payload(input: &Json) -> Result<Json, String> {
    read_back(payload_member(input)?)
}

fn payload_text(input: &Json) -> Result<Json, String> {
    let kvs = members("input", &["op", "text"], input)?;
    let raw = text("input.text", required("input", kvs, "text")?)?;
    read_back(&crate::parse_json(raw).map_err(|e| e.to_string())?)
}

fn raw_members(input: &Json) -> Result<Json, String> {
    let m = match read_tick_members(&sha256, &Rules::current(), payload_member(input)?) {
        Ok(m) => m,
        Err(f) => return Ok(refusal(f)),
    };
    Ok(Json::Obj(vec![
        ("rules".into(), Json::Str(m.rules().text())),
        ("spec".into(), m.spec().clone()),
        ("result".into(), m.result().clone()),
        ("covers".into(), tick_ids(m.covers())),
        ("withheld".into(), tick_ids(m.withheld())),
    ]))
}

fn fields(input: &Json) -> Result<Json, String> {
    crate::payload::fields(payload_member(input)?)
}

fn opened_form(path: &str, j: &Json, key: &str) -> Result<Record, String> {
    let pair = members(path, &["record", key], j)?;
    let mut r = record(&format!("{path}.record"), required(path, pair, "record")?)?;
    r.fields = payload_fields(required(path, pair, key)?);
    r.shown = Shown::Here;
    Ok(r)
}

fn read_ticks(input: &Json) -> Result<Json, String> {
    let kvs = members("input", &["op", "spec", "ticks"], input)?;
    let spec = crate::spec::parsed(required("input", kvs, "spec")?)?;
    let mut records = Vec::new();
    for (i, j) in list("input.ticks", required("input", kvs, "ticks")?)?
        .iter()
        .enumerate()
    {
        let path = format!("input.ticks[{i}]");
        records.push(opened_form(&path, j, "payload")?);
    }
    Ok(result_json(&spec.run(&records)))
}

type Operation = fn(&Json) -> Result<Json, String>;

fn operation(name: &str) -> Option<Operation> {
    match name {
        "hash" => Some(hash),
        "domains" => Some(domains),
        "window" => Some(window),
        "write-payload" => Some(write_payload),
        "payload" => Some(payload),
        "payload-text" => Some(payload_text),
        "members" => Some(raw_members),
        "make-and-check" => Some(check::make_and_check),
        "check-tick" => Some(check::check_subject),
        "fields" => Some(fields),
        "read-ticks" => Some(read_ticks),
        _ => None,
    }
}

/// Whether the evaluator can compute this tick fixture operation.
pub fn supports_tick_op(name: &str) -> bool {
    operation(name).is_some()
}

pub(crate) fn answers(layer: &str, input: &Json) -> Result<Vec<Json>, String> {
    if layer != "tick" {
        return Err(format!("unknown layer {layer}"));
    }
    let kvs = object("input", input)?;
    let name = text("input.op", required("input", kvs, "op")?)?;
    let run = operation(name).ok_or_else(|| format!("unknown tick fixture operation {name}"))?;
    Ok(vec![run(input)?])
}

#[cfg(test)]
mod tests {
    use crate::{evaluate, supports_tick_op};

    #[test]
    fn unknown_operations_and_malformed_known_operations_are_errors() {
        assert!(!supports_tick_op("future"));
        for input in [
            r#"{"op":"future"}"#,
            r#"{"op":"hash","samples":[],"unexpected":true}"#,
            r#"{"op":"hash","samples":[{"rules":1,"spec":{}}]}"#,
            r#"{"op":"domains","rules":"1","spec":{}}"#,
            r#"{"op":"window","records":[]}"#,
        ] {
            assert!(
                evaluate(&format!(r#"{{"layer":"tick","input":{input}}}"#)).is_err(),
                "{input}"
            );
        }
    }

    #[test]
    fn raw_member_hash_uses_the_models_jcs_for_an_overflow_in_a_carried_note() {
        let vector = r#"{"layer":"tick","input":{"op":"hash","samples":[
            {"rules":"1","spec":{"vertex":"x","from":[{"store":"."}],"note":[1e999]}}]}}"#;
        assert_eq!(
            evaluate(vector).expect("carried number"),
            evaluate(&vector.replace("1e999", "null")).expect("canonical null")
        );
    }
}
