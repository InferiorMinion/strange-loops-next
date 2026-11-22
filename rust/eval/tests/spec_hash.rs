//! Direct checks of spec identity and the window field in model/Gen/Checks.lean.
use loops::{
    base32,
    families::Tri,
    json::Json,
    occurred::{Claim, Stamp},
    record::Record,
    spec::Spec,
    spec_hash::{Rules, spec_hash_of},
    where_::{Arg, Cond, Field, Op, RefusalCode},
};
use loops_eval::parse_json;
use sha2::{Digest, Sha256};

fn sha256(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

#[test]
fn rules_are_positive_canonical_decimal_and_compare_numerically() {
    let read = |s: &str| Rules::of_json(&Json::Str(s.into()));
    let big = "123456789012345678901234567890123456789012345678901234567890";
    for text in ["1", "2", "10", big] {
        assert_eq!(read(text).expect("version").text(), text);
    }
    assert_eq!(read("1"), Some(Rules::current()));
    assert!(read("10").expect("ten") > read("2").expect("two"));
    for text in ["0", "01", "", "1 ", " 1", "+1", "١", "1_2"] {
        assert_eq!(read(text), None, "{text:?}");
    }
    assert_eq!(Rules::of_json(&Json::Int(1_u64.into())), None);
}

#[test]
fn the_parsed_backups_hash_binds_the_rules_version() {
    let raw = parse_json(
        r#"{"vertex":"backups","from":[{"store":"lab","kind":["backup"]}],
        "fold":{"runs":{"count":true},"mb":{"sum":"mb"},"list":{"collect":true}}}"#,
    )
    .expect("JSON");
    let backups = Spec::of_json(&raw).expect("backups");
    let hash = backups.hash(&sha256, &Rules::current());
    assert_eq!(
        hash,
        "spec:gs9soetlnq1fqp17th66fedrl0oecag0uf101fm5ov2c46ss73ag"
    );
    let two = Rules::of_json(&Json::Str("2".into())).expect("version");
    assert_ne!(backups.hash(&sha256, &two), hash);
}

#[test]
fn a_future_spec_member_can_be_hashed_before_parsing() {
    let raw = parse_json(r#"{"future":true}"#).expect("JSON");
    assert!(Spec::of_json(&raw).is_err());
    let digest = sha256(br#"loops/spec-hash/v1:{"future":true}"#);
    assert_eq!(
        spec_hash_of(&sha256, &Rules::current(), &raw),
        format!("spec:{}", base32::base32(&digest))
    );
}

#[test]
fn when_reads_the_envelope_for_every_time_operator() {
    let today = Claim::of("2026-11-21").expect("day");
    let tomorrow = Claim::of("2026-11-22").expect("day");
    let record = Record {
        occurred: Some(today.clone()),
        emitted: Stamp::of("2026-11-22T00:00:00Z").expect("timestamp"),
        ..Record::default()
    };
    let cond = |op| Cond {
        field: Field::of_name("env.when").expect("known field"),
        op,
    };
    for hidden in [record.withhold(), record.erase()] {
        for (op, answer) in [
            (Op::Is(Arg::Time(today.clone())), Tri::Yes),
            (Op::Not(Arg::Time(today.clone())), Tri::No),
            (Op::Present, Tri::Yes),
            (Op::Absent, Tri::No),
            (Op::AtLeast(Arg::Time(today.clone())), Tri::Yes),
            (Op::Below(Arg::Time(tomorrow.clone())), Tri::Yes),
        ] {
            assert_eq!(cond(op).eval(&hidden), answer);
        }
        let fallback = Record {
            occurred: None,
            ..hidden
        };
        assert_eq!(
            cond(Op::Below(Arg::Time(tomorrow.clone()))).eval(&fallback),
            Tri::No
        );
        let instant = Claim::of(fallback.emitted.text()).expect("instant");
        assert_eq!(cond(Op::Is(Arg::Time(instant))).eval(&fallback), Tri::Yes);
    }
}

#[test]
fn when_is_refused_as_a_group_key() {
    let raw = parse_json(
        r#"{"vertex":"x","from":[{"store":"."}],"fold":{"n":{"count":true,"by":"env.when"}}}"#,
    )
    .expect("JSON");
    let refusal = Spec::of_json(&raw).expect_err("by takes grouping fields");
    assert_eq!(refusal.code, RefusalCode::ByInvalid);
    assert_eq!(refusal.path, "fold.n.by");
}
