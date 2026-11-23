//! Dependency selection over invented holdings, independently folded at both sizes.
use loops::{
    record::{Digest, Held, Record, Val, id_text},
    result::{result_hash, result_text},
    slice::{Closure, spec_slice},
    spec::{By, FoldType, Follow, Slot, Source, Spec},
    things::settle,
    where_::{Field, Op, Tests},
};
use sha2::{Digest as _, Sha256};

fn id(n: u8) -> Digest {
    Digest([n; 32])
}

fn record(n: u8, kind: &str, thing: &str, prev: &[u8]) -> Record {
    Record {
        id: id(n),
        kind: kind.into(),
        of: Some(thing.into()),
        prev: prev.iter().copied().map(id).collect(),
        ..Record::default()
    }
}

fn slot(name: &str, fold_type: FoldType) -> Slot {
    Slot {
        name: name.into(),
        fold_type,
        tests: Tests::default(),
        group: None,
    }
}

fn count(follow: Option<Follow>) -> Slot {
    slot("runs", FoldType::Count { follow })
}

fn sum(field: &str, follow: Option<Follow>) -> Slot {
    slot(
        field,
        FoldType::Sum {
            field: field.into(),
            follow,
        },
    )
}

fn spec(kind: &str, slots: Vec<Slot>) -> Spec {
    Spec {
        vertex: "workshop".into(),
        sources: vec![Source {
            store: ".".into(),
            kinds: vec![kind.into()],
            ..Source::default()
        }],
        slots,
    }
}

fn answer(spec: &Spec, forms: &[Record]) -> String {
    result_text(&spec.run(&settle(forms)))
}

// Exercise the public boundary on a settled holding. As a tick would, project
// the IDs back to records with the available openings, then settle and admit
// afresh inside the projection. Expected IDs are independent of this fold.
fn check(spec: &Spec, forms: &[Record], exact: &[u8], downward: &[u8]) -> String {
    let whole = settle(forms);
    let result = spec.run(&whole);
    let bytes = result_text(&result);
    let sha256 = |input: &[u8]| Sha256::digest(input).to_vec();
    let hash = result_hash(&sha256, &result);
    for (closure, expected) in [(Closure::Exact, exact), (Closure::Downward, downward)] {
        let ids = spec_slice(spec, &whole, closure);
        assert_eq!(
            ids,
            expected.iter().copied().map(id).collect::<Vec<_>>(),
            "{closure:?}"
        );
        let part: Vec<_> = whole
            .iter()
            .filter(|r| ids.contains(&r.id))
            .cloned()
            .collect();
        let result = spec.run(&settle(&part));
        assert_eq!(
            result_text(&result).as_bytes(),
            bytes.as_bytes(),
            "{closure:?}"
        );
        assert_eq!(result_hash(&sha256, &result), hash, "{closure:?}");
    }
    bytes
}

#[test]
fn retired_successor_changes_current_backups() {
    let current = spec(
        "backup",
        vec![count(Some(Follow::Prev)), sum("mb", Some(Follow::Prev))],
    );
    let mut records: Vec<_> = [(1, 412_i64), (2, 3), (3, 88), (4, 400), (5, 4), (6, 120)]
        .into_iter()
        .map(|(n, mb)| Record {
            fields: vec![("mb".into(), Val::Int(mb.into()).into())],
            ..record(n, "backup", &format!("backup:bench-{n}"), &[])
        })
        .collect();
    assert_eq!(
        answer(&current, &records),
        r#"{"mb":{"unread":"0","value":"1027"},"runs":{"unread":"0","value":"6"}}"#
    );
    records.push(record(7, "retired", "backup:bench-1", &[1]));
    records.push(record(8, "note", "yard:gate", &[]));
    records.reverse();
    let seven = [1, 2, 3, 4, 5, 6, 7];
    assert_eq!(
        check(&current, &records, &seven, &seven),
        r#"{"mb":{"unread":"0","value":"615"},"runs":{"unread":"0","value":"5"}}"#
    );
}

#[test]
fn eraser_of_taken_firing_acts_on_all_forms_before_admission() {
    let firing = Record {
        fields: vec![("temperature".into(), Val::Int(900_i64.into()).into())],
        ..record(1, "firing", "kiln:east", &[])
    };
    // Another thing's predecessor refuses this eraser. It still erases.
    let eraser = Record {
        erases: vec![firing.id],
        ..record(2, "withdrawal", "yard:log", &[3])
    };
    let mut forms = vec![
        firing.withhold(),
        firing,
        record(3, "note", "workshop:log", &[]),
    ];
    let readings = spec("firing", vec![sum("temperature", None)]);
    let open = r#"{"temperature":{"unread":"0","value":"900"}}"#;
    assert_eq!(check(&readings, &forms, &[1], &[1]), open);
    forms.push(eraser);
    assert_eq!(
        check(&readings, &forms, &[1, 2], &[1, 2, 3]),
        r#"{"temperature":{"unread":"1","value":"0"}}"#
    );
    // Drop the eraser from the original forms, before settlement masks the opening.
    assert_eq!(answer(&readings, &forms[..3]), open);
}

#[test]
fn taken_records_foreign_target_keeps_the_record_refused() {
    let records = [
        record(1, "repair", "workshop:saw", &[2]),
        record(2, "note", "yard:pump", &[]),
    ];
    let repairs = spec("repair", vec![count(None)]);
    assert_eq!(
        check(&repairs, &records, &[1, 2], &[1, 2]),
        r#"{"runs":{"unread":"0","value":"0"}}"#
    );
    assert_eq!(
        answer(&repairs, &records[..1]),
        r#"{"runs":{"unread":"0","value":"1"}}"#
    );
}

#[test]
fn successors_other_target_keeps_the_taken_head_current() {
    let records = [
        record(1, "repair", "workshop:saw", &[]),
        record(2, "retired", "workshop:saw", &[1, 3]),
        record(3, "note", "yard:pump", &[]),
    ];
    let repairs = spec("repair", vec![count(Some(Follow::Prev))]);
    assert_eq!(
        check(&repairs, &records, &[1, 2, 3], &[1, 2, 3]),
        r#"{"runs":{"unread":"0","value":"1"}}"#
    );
    assert_eq!(
        answer(&repairs, &records[..2]),
        r#"{"runs":{"unread":"0","value":"0"}}"#
    );
}

#[test]
fn keyed_kiln_reads_untaken_live_setter_and_its_eraser() {
    let setter = Record {
        sets: Some(vec!["temperature".into()]),
        fields: vec![("temperature".into(), Val::Int(840_i64.into()).into())],
        ..record(1, "setup", "kiln:east", &[])
    };
    let join = Record {
        sets: Some(vec![]),
        ..record(3, "join", "kiln:east", &[1, 2])
    };
    let mut records = vec![
        setter,
        Record {
            sets: Some(vec!["fuel".into()]),
            ..record(2, "setup", "kiln:east", &[])
        },
        join,
        Record {
            sets: Some(vec!["batch".into()]),
            ..record(4, "firing", "kiln:east", &[3])
        },
        record(5, "note", "yard:pump", &[]),
    ];
    let readings = spec(
        "firing",
        vec![count(None), sum("temperature", Some(Follow::Keys))],
    );
    let members = [1, 2, 3, 4];
    assert_eq!(
        check(&readings, &records, &members, &members),
        r#"{"runs":{"unread":"0","value":"1"},"temperature":{"unread":"0","value":"840"}}"#
    );
    records.push(Record {
        erases: vec![id(1)],
        ..record(6, "withdrawal", "yard:log", &[])
    });
    let erased = [1, 2, 3, 4, 6];
    assert_eq!(
        check(&readings, &records, &erased, &erased),
        r#"{"runs":{"unread":"0","value":"1"},"temperature":{"unread":"1","value":"0"}}"#
    );
    let mut grouped = count(Some(Follow::Keys));
    grouped.tests = Tests(vec![(
        Field::Payload("temperature".into()),
        vec![Op::Present],
    )]);
    grouped.group = Some(By::Kind);
    assert_eq!(
        check(&spec("firing", vec![grouped]), &records, &erased, &erased),
        r#"{"runs":{"groups":{},"ungrouped":"0","unread":"1"}}"#
    );
}

#[test]
fn refused_keyed_member_needs_its_own_foreign_target() {
    let records = [
        Record {
            sets: Some(vec!["priority".into()]),
            ..record(1, "setup", "kiln:east", &[3])
        },
        Record {
            sets: Some(vec!["batch".into()]),
            ..record(2, "firing", "kiln:east", &[1])
        },
        record(3, "note", "yard:pump", &[]),
        record(4, "note", "yard:pump", &[3]),
    ];
    let readings = spec(
        "firing",
        vec![slot(
            "state",
            FoldType::LatestBy {
                follow: Follow::Keys,
            },
        )],
    );
    assert!(!check(&readings, &records, &[1, 2, 3], &[1, 2, 3]).contains("priority"));
    assert!(answer(&readings, &records[..2]).contains("priority"));
}

#[test]
fn keyed_history_keeps_the_bridge_between_setters() {
    let old = Record {
        sets: Some(vec!["limit".into()]),
        fields: vec![("limit".into(), Val::Int(600_i64.into()).into())],
        ..record(1, "setup", "kiln:east", &[])
    };
    let new = Record {
        sets: Some(vec!["limit".into()]),
        fields: vec![("limit".into(), Val::Int(700_i64.into()).into())],
        ..record(3, "setup", "kiln:east", &[2])
    };
    let mut forms = vec![
        old,
        new,
        Record {
            sets: Some(vec!["batch".into()]),
            ..record(4, "firing", "kiln:east", &[])
        },
        Record {
            sets: Some(vec![]),
            ..record(2, "join", "kiln:east", &[1])
        },
    ];
    let limits = spec("firing", vec![sum("limit", Some(Follow::Keys))]);
    let members = [1, 2, 3, 4];
    assert_eq!(
        check(&limits, &forms, &members, &members),
        r#"{"limit":{"unread":"0","value":"700"}}"#
    );
    forms.pop();
    assert_eq!(
        answer(&limits, &forms),
        r#"{"limit":{"unread":"1","value":"0"}}"#
    );
}

#[test]
fn keyed_cover_includes_dead_history_even_for_a_refused_source_match() {
    let records = [
        record(1, "setup", "kiln:east", &[]),
        record(2, "setup", "kiln:east", &[1]),
        record(3, "firing", "kiln:east", &[4]),
        record(4, "note", "yard:pump", &[]),
        // A successor of an untaken member, outside the touched thing.
        record(5, "note", "yard:gate", &[1]),
    ];
    let readings = spec("firing", vec![count(Some(Follow::Keys))]);
    assert_eq!(
        check(&readings, &records, &[1, 2, 3, 4], &[1, 2, 3, 4]),
        r#"{"runs":{"unread":"0","value":"0"}}"#
    );
}

#[test]
fn taken_tick_does_not_expand_payload_covers() {
    let records = [
        Record {
            fields: vec![("covers".into(), Held::Many(vec![Val::Str(id_text(&id(2)))]))],
            ..record(1, "tick", "workshop:weekly", &[])
        },
        record(2, "backup", "backup:workshop", &[]),
    ];
    let ticks = spec("tick", vec![count(None)]);
    assert_eq!(
        check(&ticks, &records, &[1], &[1]),
        r#"{"runs":{"unread":"0","value":"1"}}"#
    );
}

#[test]
fn empty_selection_and_matching_source_without_slots() {
    let records = [
        record(1, "setup", "kiln:east", &[]),
        record(2, "note", "yard:gate", &[]),
    ];
    let firings = spec("firing", vec![count(Some(Follow::Keys))]);
    assert_eq!(
        check(&firings, &records, &[], &[]),
        r#"{"runs":{"unread":"0","value":"0"}}"#
    );
    assert_eq!(check(&spec("setup", vec![]), &records, &[1], &[1]), "{}");
}

#[test]
fn sources_choose_the_slice_before_slot_tests() {
    let mut filtered = count(None);
    filtered.tests = Tests(vec![(Field::Payload("finished".into()), vec![Op::Present])]);
    let mut readings = spec("setup", vec![filtered]);
    readings.sources[0].about = Some("yard:shed".into());
    readings.sources.push(Source {
        of: Some(id_text(&id(4))),
        store: ".".into(),
        ..Source::default()
    });
    let records = [
        Record {
            about: vec!["yard:shed".into()],
            ..record(1, "setup", "kiln:east", &[])
        },
        record(2, "retired", "kiln:east", &[1]),
        record(3, "setup", "kiln:west", &[]),
        Record {
            of: None,
            ..record(4, "note", "yard:shed", &[])
        },
    ];
    assert_eq!(
        check(&readings, &records, &[1, 2, 4], &[1, 2, 4]),
        r#"{"runs":{"unread":"0","value":"0"}}"#
    );
}

#[test]
fn downward_follows_held_prev_and_under_recursively_inside_the_input() {
    let records = [
        record(1, "setup", "kiln:east", &[2, 99]),
        Record {
            under: Some(id(4)),
            ..record(2, "note", "kiln:east", &[3])
        },
        record(3, "note", "kiln:east", &[]),
        record(4, "grant", "workshop:permits", &[5]),
        Record {
            under: Some(id(6)),
            ..record(5, "grant", "workshop:permits", &[])
        },
        Record {
            erases: vec![id(7)],
            ..record(6, "permit", "yard:permits", &[])
        },
        record(7, "note", "yard:gate", &[]),
    ];
    let setups = spec("setup", vec![count(None)]);
    check(&setups, &records, &[1, 2], &[1, 2, 3, 4, 5, 6]);
    // A cut lacks the grant's parent. Its named ID cannot extend this holding.
    check(&setups, &records[..4], &[1, 2], &[1, 2, 3, 4]);
}
