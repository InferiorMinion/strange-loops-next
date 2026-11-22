//! Public checker behavior independent of fixture dispatch and verdict JSON.
use loops::{
    json::Json,
    record::{Record, payload_fields},
    result::result_hash_of,
    spec::Spec,
    spec_hash::{Rules, spec_hash_of},
    tick::{make_tick, tick_ids},
    tick_check::{
        CoverClosure, MissingInputs, ReplayIssue, TickAnswer, TickCheck, TickSubject, TodayReport,
        check_tick, tick_open,
    },
    where_::{Refusal, RefusalCode},
};
use loops_eval::parse_json;
use sha2::{Digest, Sha256};
use std::slice::from_ref;

fn sha(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

fn spec() -> Spec {
    let json = parse_json(
        r#"{"vertex":"records","from":[{"store":"."}],"fold":{
        "n":{"count":true},"ids":{"collect":true}}}"#,
    )
    .expect("spec JSON");
    Spec::of_json(&json).expect("count and list spec")
}

fn record(n: u8) -> Record {
    Record {
        id: [n; 32].into(),
        kind: "note".into(),
        ..Record::default()
    }
}

fn two() -> Rules {
    Rules::of_json(&Json::Str("2".into())).expect("version 2")
}

fn check(current: &Rules, raw: &Json, forms: &[Record]) -> TickCheck {
    check_tick(
        &sha,
        current,
        TickSubject {
            kind: "tick",
            payload: Some(raw),
        },
        forms,
    )
}

fn set(obj: &mut Json, key: &str, value: Json) {
    let Json::Obj(kvs) = obj else {
        panic!("expected object")
    };
    kvs.iter_mut().find(|(k, _)| k == key).expect("member").1 = value;
}

fn set_spec(raw: &mut Json, spec: Json) {
    set(
        raw,
        "spec-hash",
        Json::Str(spec_hash_of(&sha, &Rules::current(), &spec)),
    );
    set(raw, "spec", spec);
}

fn set_result(raw: &mut Json, result: Json) {
    set(raw, "result-hash", Json::Str(result_hash_of(&sha, &result)));
    set(raw, "result", result);
}

fn refused(code: RefusalCode, path: &str) -> TickAnswer {
    TickAnswer::NotTick(Refusal {
        code,
        path: path.into(),
    })
}

#[test]
fn made_tick_stays_same_with_a_late_record_and_reports_the_outside_count() {
    let writer = [record(1)];
    let raw = make_tick(spec(), &writer).to_json(&sha);
    let now = Rules::current();
    assert_eq!(
        check(&now, &raw, &writer),
        TickCheck {
            answer: TickAnswer::Same,
            closure: CoverClosure::Yes,
            taken_outside: Some(0)
        }
    );
    assert_eq!(
        check(&now, &raw, &[record(1), record(2), record(2)]),
        TickCheck {
            answer: TickAnswer::Same,
            closure: CoverClosure::Yes,
            taken_outside: Some(1)
        }
    );
}

#[test]
fn kind_opening_and_newer_rules_gate_the_reports() {
    let now = Rules::current();
    for (kind, answer) in [
        ("note", refused(RefusalCode::TickKind, "kind")),
        ("tick", TickAnswer::PayloadUnheld),
    ] {
        assert_eq!(
            check_tick(
                &sha,
                &now,
                TickSubject {
                    kind,
                    payload: None
                },
                &[]
            ),
            TickCheck {
                answer,
                closure: CoverClosure::Unknown,
                taken_outside: None
            }
        );
    }
    let mut claim = make_tick(spec(), &[]);
    claim.rules = two();
    let raw = claim.to_json(&sha);
    assert_eq!(
        check(&now, &raw, &[]),
        TickCheck {
            answer: TickAnswer::Newer,
            closure: CoverClosure::Yes,
            taken_outside: None
        }
    );
}

#[test]
fn spec_failures_distinguish_older_diagnostics_from_current_refusals() {
    let mut raw = make_tick(spec(), &[]).to_json(&sha);
    set_spec(&mut raw, Json::Obj(vec![]));
    let now = Rules::current();
    assert_eq!(check(&now, &raw, &[]).answer, TickAnswer::UnreadableSpec);
    assert_eq!(
        check(&two(), &raw, &[]),
        TickCheck {
            answer: TickAnswer::Older(TodayReport::UnreadableSpec),
            closure: CoverClosure::Yes,
            taken_outside: None
        }
    );
    let Json::Obj(mut kvs) = spec().to_json() else {
        panic!("spec object")
    };
    kvs.push(("note".into(), Json::Null));
    set_spec(&mut raw, Json::Obj(kvs));
    assert_eq!(
        check(&now, &raw, &[]).answer,
        refused(RefusalCode::TickSpecNoncanonical, "spec")
    );
    assert_eq!(
        check(&two(), &raw, &[]).answer,
        TickAnswer::Older(TodayReport::NoncanonicalSpec)
    );
}

#[test]
fn closure_and_missing_evidence_use_all_held_forms() {
    let a = record(1);
    let b = record(2);
    let now = Rules::current();
    let raw = make_tick(spec(), &[a.clone(), b]).to_json(&sha);
    let eraser = Record {
        erases: vec![a.id],
        ..record(3)
    };
    let mut forms = vec![a.clone(), a.withhold()];
    assert!(tick_open(&forms, &a.id));
    forms.push(eraser);
    assert!(!tick_open(&forms, &a.id));
    let missing = MissingInputs {
        records: 1,
        openings: 1,
    };
    assert_eq!(
        check(&now, &raw, &forms),
        TickCheck {
            answer: TickAnswer::Missing(missing),
            closure: CoverClosure::Unknown,
            taken_outside: Some(1)
        }
    );
    assert_eq!(
        check(&two(), &raw, &forms).answer,
        TickAnswer::Older(TodayReport::Unavailable(ReplayIssue::Missing(missing)))
    );
    // An envelope alone establishes an outside link even with another id missing.
    forms[0].under = Some(record(4).id);
    assert_eq!(check(&now, &raw, &forms).closure, CoverClosure::No);
}

#[test]
fn a_covered_eraser_proves_an_omission_before_missing_inputs() {
    let a = record(1);
    let eraser = Record {
        erases: vec![a.id],
        ..record(2)
    };
    let mut claim = make_tick(spec(), &[a, eraser.clone()]);
    claim.withheld.clear();
    let mut raw = claim.to_json(&sha);
    let partial = [eraser.withhold()];
    assert_eq!(
        check(&Rules::current(), &raw, &partial).answer,
        refused(RefusalCode::TickWithheldErased, "withheld")
    );
    let older = TickAnswer::Older(TodayReport::Unavailable(ReplayIssue::IgnoredErasure));
    assert_eq!(check(&two(), &raw, &partial).answer, older);
    set_result(&mut raw, Json::Null);
    assert_eq!(
        check(&Rules::current(), &raw, &partial).answer,
        refused(RefusalCode::TickResultInvalid, "result")
    );
    assert_eq!(check(&two(), &raw, &partial).answer, older);
}

#[test]
fn wrong_and_unreconciled_claims_report_both_hashes() {
    let forms = [record(1)];
    let claim = make_tick(spec(), &forms);
    let mut raw = claim.to_json(&sha);
    let result = loops::result::result_json(&claim.result);
    let computed = result_hash_of(&sha, &result);
    assert_eq!(
        check(&two(), &raw, &forms).answer,
        TickAnswer::Older(TodayReport::Same)
    );
    // A list of the claimed length naming the wrong id, then a list shorter than the count.
    for heads in [tick_ids(&[record(2).id]), tick_ids(&[])] {
        let Json::Obj(mut slots) = result.clone() else {
            panic!("slots")
        };
        let entry = &mut slots
            .iter_mut()
            .find(|(k, _)| k == "ids")
            .expect("ids slot")
            .1;
        set(entry, "heads", heads);
        let wrong = Json::Obj(slots);
        let claimed = result_hash_of(&sha, &wrong);
        assert_ne!(claimed, computed);
        set_result(&mut raw, wrong);
        assert_eq!(
            check(&Rules::current(), &raw, &forms).answer,
            TickAnswer::Differs {
                claimed,
                computed: computed.clone()
            }
        );
        assert_eq!(
            check(&two(), &raw, &forms).answer,
            TickAnswer::Older(TodayReport::Another)
        );
    }
}

#[test]
fn equal_hashes_do_not_make_unequal_result_bytes_agree() {
    let hash = |_: &[u8]| vec![0; 32];
    let mut raw = make_tick(spec(), &[]).to_json(&hash);
    let wrong = parse_json(r#"{"n":{"value":"1","unread":"0"},"ids":{"heads":[],"unread":"0"}}"#)
        .expect("result");
    let digest = result_hash_of(&hash, &wrong);
    set(&mut raw, "result", wrong);
    let check = check_tick(
        &hash,
        &Rules::current(),
        TickSubject {
            kind: "tick",
            payload: Some(&raw),
        },
        &[],
    );
    assert_eq!(
        check.answer,
        TickAnswer::Differs {
            claimed: digest.clone(),
            computed: digest
        }
    );
}

#[test]
fn a_covered_tick_is_one_input_even_when_its_own_cover_is_absent() {
    let inner = make_tick(spec(), &[record(1), record(2)]).to_json(&sha);
    let nested = Record {
        kind: "tick".into(),
        fields: payload_fields(&inner),
        ..record(3)
    };
    let outer = make_tick(spec(), from_ref(&nested));
    assert_eq!(outer.covers, [nested.id]);
    let result = loops::result::result_json(&outer.result);
    let Json::Obj(slots) = result else {
        panic!("slots")
    };
    assert_eq!(
        slots
            .iter()
            .find(|(k, _)| k == "n")
            .expect("count")
            .1
            .canon(),
        r#"{"unread":"0","value":"1"}"#
    );
    assert_eq!(
        check(&Rules::current(), &outer.to_json(&sha), from_ref(&nested)).answer,
        TickAnswer::Same
    );
}
