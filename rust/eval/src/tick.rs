//! Input-only answers for the operations in model/Gen/Tick.lean.
use crate::reader::{list, members, object, record, required, text};
use loops::{
    json::Json,
    result::{result_json, result_preimage},
    spec_hash::{Rules, spec_hash_of, spec_label, spec_preimage_of},
};
use sha2::{Digest, Sha256};

fn read_rules(j: &Json) -> Result<Rules, String> {
    Rules::of_json(j).ok_or_else(|| "rules: expected canonical positive decimal text".into())
}

fn hash(input: &Json) -> Result<Json, String> {
    let kvs = members("input", &["op", "samples"], input)?;
    let samples = list("input.samples", required("input", kvs, "samples")?)?;
    let sha256 = |bytes: &[u8]| Sha256::digest(bytes).to_vec();
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

fn window(input: &Json) -> Result<Json, String> {
    let kvs = members("input", &["op", "spec", "records"], input)?;
    let spec = crate::spec::parsed(required("input", kvs, "spec")?)?;
    let records = list("input.records", required("input", kvs, "records")?)?
        .iter()
        .enumerate()
        .map(|(i, r)| record(&format!("input.records[{i}]"), r))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(result_json(&spec.run(&records)))
}

type Operation = fn(&Json) -> Result<Json, String>;

fn operation(name: &str) -> Option<Operation> {
    match name {
        "hash" => Some(hash),
        "domains" => Some(domains),
        "window" => Some(window),
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
