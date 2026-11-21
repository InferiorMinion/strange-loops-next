//! The canonical result (D7) against model/Loops/Result.lean and protocol/result.md.
use loops::{
    json::Json,
    record::{Digest, Record, Val},
    result::{RESULT_LABEL, result_hash, result_preimage, result_text},
    spec::Spec,
};
use sha2::{Digest as _, Sha256};

fn spec(raw: &str) -> Spec {
    Spec::of_json(&loops_eval::parse_json(raw).expect("test JSON")).expect("valid spec")
}
fn rec(n: u8, thing: &str) -> Record {
    let mut bytes = [0; 32];
    bytes[31] = n;
    Record {
        id: Digest(bytes),
        kind: "task".into(),
        observer: "sl:alice".into(),
        of: Some(thing.into()),
        ..Record::default()
    }
}
fn with_n(n: u8, thing: &str, value: i64) -> Record {
    let mut r = rec(n, thing);
    r.fields.push(("n".into(), Val::Int(value.into())));
    r
}
fn sha256(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}
const NUMBERS: &str = r#"{"vertex":"x","from":[{"store":"."}],"fold":{"sum":{"sum":"n"},"min":{"min":"n"},"max":{"max":"n"}}}"#;

#[test]
fn a_sum_past_2_53_keeps_every_digit() {
    let xs = [
        with_n(1, "task:1", 9_007_199_254_740_991),
        with_n(2, "task:2", 2),
    ];
    assert_eq!(
        result_text(&spec(NUMBERS).run(&xs)),
        r#"{"max":{"unread":"0","value":"9007199254740991"},"min":{"unread":"0","value":"2"},"sum":{"unread":"0","value":"9007199254740993"}}"#
    );
}

#[test]
fn a_negative_past_2_53_and_a_sum_of_zero() {
    let least = -9_007_199_254_740_991;
    let xs = [
        with_n(1, "task:1", least),
        with_n(2, "task:2", least),
        with_n(3, "task:3", -2),
    ];
    assert_eq!(
        result_text(&spec(NUMBERS).run(&xs)),
        r#"{"max":{"unread":"0","value":"-2"},"min":{"unread":"0","value":"-9007199254740991"},"sum":{"unread":"0","value":"-18014398509481984"}}"#
    );
    let xs = [with_n(1, "task:1", 5), with_n(2, "task:2", -5)];
    assert!(result_text(&spec(NUMBERS).run(&xs)).contains(r#""sum":{"unread":"0","value":"0"}"#));
}

#[test]
fn min_and_max_that_read_nothing_are_null() {
    assert_eq!(
        result_text(&spec(NUMBERS).run(&[])),
        r#"{"max":{"unread":"0","value":null},"min":{"unread":"0","value":null},"sum":{"unread":"0","value":"0"}}"#
    );
}

#[test]
fn three_holdings_that_differ_in_shown_encode_alike() {
    let listing = spec(
        r#"{"vertex":"x","from":[{"store":".","kind":["task"]}],"fold":{
            "current":{"latest-by":"of","follow":"prev"},"every":{"collect":true},
            "state":{"latest-by":"of","follow":"keys"},"per-kind":{"collect":true,"by":"env.kind"},
            "n":{"count":true}}}"#,
    );
    let a = rec(1, "task:1");
    let mut b = rec(2, "task:1");
    b.prev = vec![a.id];
    b.fields = vec![("status".into(), loops::record::Val::Str("closed".into()))];
    let c = rec(3, "task:2");
    let mut eraser = rec(4, "note:1");
    eraser.kind = "erasure".into();
    eraser.erases = vec![b.id];
    let held = listing.run(&[a.clone(), b.clone(), c.clone()]);
    let withheld = listing.run(&[a.clone(), b.withhold(), c.clone()]);
    let after_erasure = listing.run(&[a, b.erase(), c, eraser]);
    assert_ne!(held, withheld);
    assert_ne!(held, after_erasure);
    for other in [&withheld, &after_erasure] {
        assert_eq!(result_text(&held), result_text(other));
        assert_eq!(result_hash(&sha256, &held), result_hash(&sha256, other));
    }
    assert!(!result_text(&held).contains("shown"));
}

#[test]
fn a_result_hash_is_no_record_commitment() {
    // A commitment hashes a 16-byte salt, then JCS. Past 16 bytes, the
    // preimage must be no JSON, or a record could commit to it.
    assert_eq!(RESULT_LABEL.len(), 21);
    let r = spec(NUMBERS).run(&[with_n(1, "task:1", 3)]);
    let preimage = result_preimage(&r);
    let past16 = std::str::from_utf8(&preimage[16..]).expect("UTF-8");
    assert!(loops_eval::parse_json(past16).is_err());
    let payload = loops_eval::parse_json(&result_text(&r)).expect("canonical text is JSON");
    let commit = loops::stored::commit_of(&sha256, &preimage[..16], &payload);
    assert_ne!(
        commit.strip_prefix("h:").expect("commitment prefix"),
        result_hash(&sha256, &r)
            .strip_prefix("res:")
            .expect("result hash prefix")
    );
    // With the 16-byte label of the first draft, the two were one digest.
    let old = "loops/result/v1:";
    assert_eq!(old.len(), 16);
    let mut old_preimage = old.as_bytes().to_vec();
    old_preimage.extend_from_slice(result_text(&r).as_bytes());
    assert_eq!(
        loops::stored::commit_of(&sha256, old.as_bytes(), &payload),
        format!("h:{}", loops::base32::base32(&sha256(&old_preimage)))
    );
    assert!(matches!(payload, Json::Obj(_)));
}
