//! The public tick readers and their wire faults, independently of fixture dispatch.
use loops::{
    json::Json,
    record::Shown,
    result::{read_result, result_hash_of, result_json},
    spec::SlotValue,
    spec_hash::Rules,
    tick::{TickReadError, read_tick_members, read_tick_payload},
};
use loops_eval::parse_json;
use sha2::{Digest, Sha256};

fn sha256(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

fn member<'a>(obj: &'a Json, name: &str) -> &'a Json {
    let Json::Obj(kvs) = obj else {
        panic!("expected object")
    };
    &kvs.iter()
        .find(|(k, _)| k == name)
        .expect("fixture member")
        .1
}

fn member_mut<'a>(obj: &'a mut Json, name: &str) -> &'a mut Json {
    let Json::Obj(kvs) = obj else {
        panic!("expected object")
    };
    &mut kvs
        .iter_mut()
        .find(|(k, _)| k == name)
        .expect("fixture member")
        .1
}

fn input_payload(source: &str) -> Json {
    let fixture = parse_json(source).expect("fixture JSON");
    let input = member(&fixture, "input");
    if member(input, "op") == &Json::Str("payload-text".into()) {
        let Json::Str(raw) = member(input, "text") else {
            panic!("payload text")
        };
        parse_json(raw).expect("JSON preserving repeated names")
    } else {
        member(input, "payload").clone()
    }
}

fn six_backups() -> (Json, String) {
    let fixture =
        parse_json(include_str!("../../../vectors/tick/six-backups.json")).expect("fixture");
    let want = member(&fixture, "expect");
    let Json::Str(jcs) = member(want, "jcs") else {
        panic!("expected JCS")
    };
    (member(want, "payload").clone(), jcs.clone())
}

#[test]
fn six_backups_reads_and_reencodes_byte_for_byte() {
    let (raw, jcs) = six_backups();
    let checked = read_tick_members(&sha256, &raw).expect("wire members");
    assert_eq!(checked.to_json(&sha256), raw);
    let payload = checked
        .read_payload(&Rules::current())
        .expect("current spec");
    assert_eq!(payload.covers.len(), 6);
    assert_eq!(payload.withheld, []);
    let (_, SlotValue::Heads { heads, .. }) = payload
        .result
        .iter()
        .find(|(k, _)| k == "list")
        .expect("list slot")
    else {
        panic!("expected listed ids")
    };
    assert_eq!(heads.len(), 6);
    assert!(heads.iter().all(|(_, shown)| *shown == Shown::Here));
    assert_eq!(payload.bytes(&sha256), jcs.as_bytes());
    assert_eq!(payload.to_json(&sha256), raw);
}

#[test]
fn reordered_payload_and_result_members_read_to_the_same_value() {
    let (mut raw, _) = six_backups();
    let read =
        |j: &Json| read_tick_payload(&sha256, &Rules::current(), j).expect("current payload");
    let expected = read(&raw);
    let Json::Obj(slots) = member_mut(&mut raw, "result") else {
        panic!("slots")
    };
    slots.reverse();
    let Json::Obj(members) = &mut raw else {
        panic!("payload")
    };
    members.reverse();
    assert_eq!(read(&raw), expected);
}

#[test]
fn six_runs_can_list_five_ids() {
    let (mut raw, _) = six_backups();
    let parsed = read_tick_payload(&sha256, &Rules::current(), &raw).expect("original");
    let result = member_mut(&mut raw, "result");
    let Json::Arr(ids) = member_mut(member_mut(result, "list"), "heads") else {
        panic!("heads")
    };
    ids.pop();
    assert_eq!(ids.len(), 5);
    assert_eq!(
        member(member(result, "runs"), "value"),
        &Json::Str("6".into())
    );
    let claim = read_result(&parsed.spec, result).expect("well-formed result");
    assert_eq!(result_json(&claim), *result);
    let hash = result_hash_of(&sha256, result);
    *member_mut(&mut raw, "result-hash") = Json::Str(hash);
    let payload = read_tick_payload(&sha256, &Rules::current(), &raw).expect("unreconciled claim");
    assert_eq!(payload.result, claim);
}

#[test]
fn another_version_is_reported_after_both_hashes_check() {
    let mut raw = input_payload(include_str!(
        "../../../vectors/tick/newer-rules-deferred.json"
    ));
    let checked = read_tick_members(&sha256, &raw).expect("opaque members");
    assert_eq!(checked.rules().text(), "2");
    assert_eq!(
        checked.read_payload(&Rules::current()),
        Err(TickReadError::OtherRules(
            Rules::of_json(&Json::Str("2".into())).expect("rules 2")
        ))
    );
    *member_mut(&mut raw, "result") = Json::Null;
    assert_eq!(
        read_tick_members(&sha256, &raw)
            .expect_err("changed result")
            .code(),
        "tick-result-hash-mismatch"
    );
}

#[test]
fn a_hash_checked_spec_can_be_unreadable() {
    let raw = input_payload(include_str!(
        "../../../vectors/tick/unreadable-spec-fitting-hash.json"
    ));
    assert_eq!(
        read_tick_members(&sha256, &raw)
            .expect("hashes")
            .read_payload(&Rules::current()),
        Err(TickReadError::UnreadableSpec)
    );
}

#[test]
fn both_hash_spellings_precede_raw_hash_comparisons() {
    let (mut raw, _) = six_backups();
    *member_mut(&mut raw, "spec") = Json::Obj(vec![]);
    *member_mut(&mut raw, "result-hash") = Json::Str("res:BAD".into());
    let fault = wire_fault(&raw);
    assert_eq!(
        (fault.code(), fault.path.as_str()),
        ("tick-hash-invalid", "result-hash")
    );
}

#[test]
fn members_refuses_a_fixture_whose_answer_cannot_be_echoed_canonically() {
    let mut raw = input_payload(include_str!(
        "../../../vectors/tick/newer-rules-members.json"
    ));
    let result = parse_json(r#"{"n":1,"n":2}"#).expect("repeated member");
    *member_mut(&mut raw, "result-hash") = Json::Str(result_hash_of(&sha256, &result));
    *member_mut(&mut raw, "result") = result;
    assert!(read_tick_members(&sha256, &raw).is_ok());
    let input = Json::Obj(vec![
        ("op".into(), Json::Str("members".into())),
        ("payload".into(), raw),
    ]);
    assert!(loops_eval::answers("tick", &input).is_err());
}

#[test]
fn overflow_in_a_supplied_payload_is_a_fixture_error() {
    for input in [
        r#"{"op":"payload","payload":{"result":1e999}}"#,
        r#"{"op":"members","payload":{"spec":{"n":-1e999}}}"#,
        r#"{"op":"payload-text","text":"{\"result\":1e999}"}"#,
        r#"{"op":"read-ticks","spec":{"vertex":"x","from":[{"store":"."}]},"ticks":[{"payload":1e999}]}"#,
    ] {
        let err = loops_eval::evaluate(&format!(r#"{{"layer":"tick","input":{input}}}"#))
            .expect_err("malformed fixture");
        assert!(err.to_string().contains("must be finite"), "{input}: {err}");
    }
}

// Pin each public refusal independently of the evaluator's JSON answer and
// verify which reader owns it.
macro_rules! refusal_case {
    ($name:ident, $file:literal, $code:literal, $path:literal, $reader:ident) => {
        #[test]
        fn $name() {
            let raw = input_payload(include_str!(concat!(
                "../../../vectors/tick/",
                $file,
                ".json"
            )));
            let error = $reader(&raw);
            assert_eq!((error.code(), error.path.as_str()), ($code, $path));
            assert_eq!(
                read_tick_payload(&sha256, &Rules::current(), &raw),
                Err(TickReadError::Invalid(error))
            );
        }
    };
}

fn wire_fault(raw: &Json) -> loops::where_::Refusal {
    read_tick_members(&sha256, raw).expect_err("wire fault")
}
fn payload_fault(raw: &Json) -> loops::where_::Refusal {
    let error = read_tick_members(&sha256, raw)
        .expect("verified members")
        .read_payload(&Rules::current())
        .expect_err("payload fault");
    let TickReadError::Invalid(f) = error else {
        panic!("expected a refusal")
    };
    f
}

refusal_case!(not_object, "not-object", "tick-not-object", "", wire_fault);
refusal_case!(
    member_duplicate,
    "duplicate-member",
    "tick-member-duplicate",
    "rules",
    wire_fault
);
refusal_case!(
    member_unknown,
    "unknown-member",
    "tick-member-unknown",
    "extra",
    wire_fault
);
refusal_case!(
    member_missing,
    "missing-spec",
    "tick-member-missing",
    "spec",
    wire_fault
);
refusal_case!(
    rules_invalid,
    "invalid-rules",
    "tick-rules-invalid",
    "rules",
    wire_fault
);
refusal_case!(
    spec_not_object,
    "spec-not-object",
    "tick-spec-not-object",
    "spec",
    wire_fault
);
refusal_case!(
    ids_not_array,
    "covers-not-array",
    "tick-ids-not-array",
    "covers",
    wire_fault
);
refusal_case!(
    id_invalid,
    "covers-invalid-id",
    "tick-id-invalid",
    "covers[0]",
    wire_fault
);
refusal_case!(
    id_duplicate,
    "covers-duplicate-id",
    "tick-id-duplicate",
    "covers",
    wire_fault
);
refusal_case!(
    ids_unsorted,
    "covers-unsorted",
    "tick-ids-unsorted",
    "covers",
    wire_fault
);
refusal_case!(
    withheld_empty,
    "withheld-empty",
    "tick-withheld-empty",
    "withheld",
    wire_fault
);
refusal_case!(
    withheld_outside,
    "withheld-outside",
    "tick-withheld-outside",
    "withheld",
    wire_fault
);
refusal_case!(
    hash_invalid,
    "result-hash-spelling",
    "tick-hash-invalid",
    "result-hash",
    wire_fault
);
refusal_case!(
    spec_hash_mismatch,
    "unreadable-spec-wrong-hash",
    "tick-spec-hash-mismatch",
    "spec-hash",
    wire_fault
);
refusal_case!(
    result_hash_mismatch,
    "result-hash-mismatch",
    "tick-result-hash-mismatch",
    "result-hash",
    wire_fault
);
refusal_case!(
    spec_noncanonical,
    "kept-note",
    "tick-spec-noncanonical",
    "spec",
    payload_fault
);
refusal_case!(
    result_invalid,
    "result-repeated-slot",
    "tick-result-invalid",
    "result",
    payload_fault
);
