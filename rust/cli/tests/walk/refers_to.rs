//! Compare every single-field reference answer with an actual keyed fold.
use super::{Json, Store, answer, at, fixed_store, parse_json, strings, text};
use std::fs;

fn describe(s: &Store, kinds: &str) {
    fs::write(
        s.dir.join(".loops/store.json"),
        format!(r#"{{"version":1,"kinds":{kinds}}}"#),
    )
    .expect("store description");
}

const KILN: &str = r#"{"kiln":{"refs":{"site":[]}}}"#;
const ONE: &str = r#"[{"store":".","kind":["kiln"]}]"#;
const GROUP: &str = r#"[{"store":"workshop","kind":["kiln"]},{"store":"yard","kind":["kiln"]}]"#;

fn group(s: &Store, yard: &Store) {
    fs::write(
        s.dir.join("stores.json"),
        format!(
            r#"{{"workshop":".","yard":{}}}"#,
            Json::Str(yard.dir.to_str().expect("yard path").into()).canon()
        ),
    )
    .expect("group file");
}

fn copy_id(from: &Store, to: &Store, id: &str) {
    to.ok_with(&["import"], &from.ok(&["get", id]));
}

fn check_field(
    s: &Store,
    sources: &str,
    target: &str,
    expected: &[&str],
    unread: usize,
    untyped: usize,
) {
    let mut args = Vec::new();
    if sources == GROUP {
        args.push("--group");
    }
    args.extend(["ls", "--refers-to", target]);
    let report = s.json(&args);
    let target_json = Json::Str(target.into()).canon();
    let spec = format!(
        r#"{{"vertex":"kiln","from":{sources},"fold":{{"sites":{{"latest-by":"of","follow":"keys","where":{{"site":{{"is":{target_json}}}}}}}}}}}"#
    );
    let fold = parse_json(&s.ok_with(&["fold", "-"], &spec)).expect("fold answer");
    let Json::Obj(things) = at(&fold, &["sites", "things"]) else {
        panic!("keyed things")
    };
    let names = strings(&report, &["things"]);
    assert_eq!(names, expected, "{target}");
    assert_eq!(
        names,
        things.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
        "{target}"
    );
    let not_read = at(&report, &["counts", "things_not_read"]);
    assert_eq!(not_read, at(&fold, &["sites", "unread"]), "{target}");
    assert_eq!(not_read.canon(), unread.to_string(), "{target}");
    assert_eq!(
        at(&report, &["counts", "values_not_typed"]).canon(),
        untyped.to_string(),
        "{target}"
    );
}

#[test]
fn forks_and_competing_lists_match_keyed_folds() {
    let s = fixed_store("kiln-reference-forks");
    describe(&s, KILN);
    let root = s.ok(&["emit", "kiln", "--of", "kiln:east", "--sets-nothing"]);
    s.changeset(&format!(
        r#"{{"kind":"kiln","of":"kiln:east","prev":["{root}"],"payload":{{"site":"kiln:a"}}}}
{{"kind":"kiln","of":"kiln:east","prev":["{root}"],"payload":{{"site":"kiln:b"}}}}"#
    ));
    for target in ["kiln:a", "kiln:b"] {
        check_field(&s, ONE, target, &[], 1, 0);
    }
    let lists = fixed_store("kiln-reference-lists");
    describe(&lists, KILN);
    lists.changeset(
        r#"{"kind":"kiln","of":"kiln:west","payload":{"site":["kiln:a","kiln:b"]}}
{"kind":"kiln","of":"kiln:west","payload":{"site":["kiln:a"]}}"#,
    );
    check_field(&lists, ONE, "kiln:a", &["kiln:west"], 0, 0);
    check_field(&lists, ONE, "kiln:b", &[], 1, 0);
    check_field(&lists, ONE, "kiln:c", &[], 0, 0);
}

#[test]
fn literal_targets_coverage_and_absent_setters_match_keyed_folds() {
    let s = fixed_store("kiln-reference-literals");
    describe(&s, r#"{"kiln":{"refs":{"site":[]}},"potter":{}}"#);
    let root = s.ok(&["emit", "potter"]);
    s.changeset(&format!(r#"{{"kind":"kiln","of":"kiln:east","payload":{{"site":"kiln:east"}}}}
{{"kind":"kiln","of":"kiln:bare","payload":{{"site":"east"}}}}
{{"kind":"kiln","of":"kiln:comma","payload":{{"site":"east,west"}}}}
{{"kind":"kiln","of":"kiln:literal","payload":{{"site":"kiln:east,kiln:west"}}}}
{{"kind":"kiln","of":"kiln:potter","payload":{{"site":"{root}"}}}}
{{"kind":"kiln","of":"kiln:missing","about":["kiln:east"],"sets":[],"payload":{{"site":"kiln:east"}}}}
{{"kind":"kiln","of":"kiln:list","payload":{{"site":["bare","bare",null,["kiln:east"],{{}}]}}}}"#));
    check_field(&s, ONE, "kiln:east", &["kiln:east"], 0, 7);
    check_field(&s, ONE, "kiln:west", &[], 0, 7);
    check_field(&s, ONE, "kiln:east,kiln:west", &["kiln:literal"], 0, 7);
    check_field(&s, ONE, &root, &["kiln:potter"], 0, 7);
    assert_eq!(s.fails(&["ls", "--refers-to", &root[..12]], "").0, 2);
}

#[test]
fn a_revision_in_another_store_removes_the_old_match_despite_its_mention() {
    let s = fixed_store("workshop-reference-move");
    let yard = fixed_store("yard-reference-move");
    let writer = fixed_store("potter-reference-move");
    group(&s, &yard);
    describe(&s, KILN);
    let a = writer.ok(&[
        "emit",
        "kiln",
        "--of",
        "kiln:east",
        "--about",
        "kiln:a",
        "site=kiln:a",
    ]);
    let b = writer.ok(&["emit", "kiln", "--prev", &a, "site=kiln:b"]);
    assert_eq!(
        strings(&writer.json(&["get", &b]), &["env", "about"]),
        ["kiln:a"]
    );
    copy_id(&writer, &s, &a);
    copy_id(&writer, &yard, &b);
    check_field(&s, ONE, "kiln:a", &["kiln:east"], 0, 0);
    check_field(&s, GROUP, "kiln:a", &[], 0, 0);
    check_field(&s, GROUP, "kiln:b", &["kiln:east"], 0, 0);
}

#[test]
fn remote_erasure_and_withheld_or_duplicate_openings_match_keyed_folds() {
    let s = fixed_store("workshop-reference-erased");
    let yard = fixed_store("yard-reference-erased");
    let writer = fixed_store("potter-reference-erased");
    group(&s, &yard);
    describe(&s, KILN);
    let id = writer.ok(&[
        "emit",
        "kiln",
        "--of",
        "kiln:east",
        r#"site:=["kiln:a","bare"]"#,
    ]);
    copy_id(&writer, &yard, &id);
    let eraser = writer.json(&["erase", &id]);
    copy_id(&writer, &s, &id); // Bare commitment only.
    check_field(&s, ONE, "kiln:a", &[], 1, 0);
    check_field(&s, GROUP, "kiln:a", &["kiln:east"], 0, 1);
    copy_id(&writer, &s, text(&eraser, &["id"]));
    check_field(&s, GROUP, "kiln:a", &[], 1, 0);
}

#[test]
fn only_current_head_kinds_select_fields_and_overlapping_roles_count_once() {
    let s = fixed_store("workshop-reference-head-kinds");
    describe(
        &s,
        r#"{"workshop":{"refs":{"site":[]}},"yard":{"refs":{"site":[]}},"potter":{}}"#,
    );
    let root = s.ok(&[
        "emit",
        "potter",
        "--of",
        "potter:east",
        r#"site:=["kiln:a","bare"]"#,
    ]);
    s.changeset(&format!(
        r#"{{"kind":"yard","of":"potter:east","prev":["{root}"],"sets":[],"payload":{{}}}}
{{"kind":"workshop","of":"potter:east","prev":["{root}"],"sets":[],"payload":{{}}}}"#
    ));
    let old = s.ok(&["emit", "yard", "--of", "potter:hidden", "site=kiln:a"]);
    s.ok(&["emit", "potter", "--prev", &old, "--sets-nothing"]);
    check_field(
        &s,
        r#"[{"store":".","kind":["workshop","yard"]}]"#,
        "kiln:a",
        &["potter:east"],
        0,
        1,
    );
}

#[test]
fn union_of_declarations_reports_provenance_and_yes_wins_over_unknown() {
    let s = fixed_store("workshop-reference-fields");
    let yard = fixed_store("yard-reference-fields");
    group(&s, &yard);
    describe(&s, r#"{"kiln":{"refs":{"site":[],"owner":[]}}}"#);
    describe(
        &yard,
        r#"{"kiln":{"refs":{"owner":[],"peer":["yard"]}},"potter":{"refs":{"yard":[]}}}"#,
    );
    s.changeset(r#"{"kind":"kiln","of":"kiln:one","sets":["peer","owner"],"payload":{"peer":"kiln:b","owner":["kiln:a","bare"]}}
{"kind":"kiln","of":"kiln:one","sets":["peer"],"payload":{"peer":"kiln:a"}}
{"kind":"kiln","of":"kiln:two","payload":{"peer":"kiln:a"}}"#);
    let args = ["--group", "ls", "--refers-to", "kiln:a"];
    let raw = s.ok(&args);
    assert_eq!(
        raw,
        s.ok(&[
            "--in",
            "yard",
            "--in",
            "workshop",
            "--in",
            "yard",
            "ls",
            "--json",
            "--refers-to",
            "kiln:a"
        ])
    );
    let report = parse_json(&raw).expect("report");
    assert_eq!(raw, report.canon());
    assert_eq!(strings(&report, &["things"]), ["kiln:one", "kiln:two"]);
    assert_eq!(
        at(&report, &["counts"]).canon(),
        r#"{"fields_read":4,"things_not_read":0,"values_not_typed":1}"#
    );
    assert_eq!(text(&report, &["target"]), "kiln:a");
    let shop_path = s.dir.to_str().expect("shop path");
    let yard_path = yard.dir.to_str().expect("yard path");
    assert_eq!(strings(&report, &["stores"]), [shop_path, yard_path]);
    let shop_file = format!("{shop_path}/.loops/store.json");
    let yard_file = format!("{yard_path}/.loops/store.json");
    let Json::Arr(rows) = at(&report, &["fields"]) else {
        panic!("fields array")
    };
    let expected: [(&str, &str, Vec<&str>); 4] = [
        ("kiln", "owner", vec![&shop_file, &yard_file]),
        ("kiln", "peer", vec![&yard_file]),
        ("kiln", "site", vec![&shop_file]),
        ("potter", "yard", vec![&yard_file]),
    ];
    assert_eq!(rows.len(), expected.len());
    for (row, (kind, field, paths)) in rows.iter().zip(expected) {
        assert_eq!(text(row, &["kind"]), kind);
        assert_eq!(text(row, &["field"]), field);
        assert_eq!(strings(row, &["descriptions"]), paths);
    }
    let plain = s.ok(&[&args[..], &["--plain"]].concat());
    for line in [
        "target  kiln:a",
        "things  2",
        "  kiln:one",
        "  kiln:two",
        "stores  2",
        "things_not_read  0",
        "values_not_typed  1",
        "fields_read  4",
    ] {
        assert!(plain.lines().any(|l| l == line), "{plain}");
    }
    for (field, path) in [
        ("owner", &shop_file),
        ("owner", &yard_file),
        ("site", &shop_file),
        ("peer", &yard_file),
    ] {
        assert!(
            plain
                .lines()
                .any(|l| l.contains(field) && l.ends_with(path)),
            "{plain}"
        );
    }
    let hidden = s.ok(&[
        "emit",
        "kiln",
        "--of",
        "kiln:hidden",
        "site=kiln:a",
        "peer=kiln:a",
    ]);
    s.ok(&["erase", &hidden]);
    let with_hidden = s.json(&args);
    assert_eq!(at(&with_hidden, &["things"]), at(&report, &["things"]));
    assert_eq!(
        at(&with_hidden, &["counts", "things_not_read"]).canon(),
        "1"
    );
}

#[test]
fn selectors_and_ls_operands_are_checked_before_open_and_empty_scopes_are_explicit() {
    let s = Store::scratch("workshop-reference-usage");
    for operands in [
        &["--refers-to"][..],
        &["--refers-to", "bare"],
        &["--refers-to", "kiln:"],
        &["--refers-to", "kiln:a", "--things"],
        &["kiln:", "--refers-to", "kiln:a"],
        &["--fields", "site", "--refers-to", "kiln:a"],
        &["--refers-to", "kiln:a", "--active"],
        &["--refers-to", "kiln:a", "--refers-to", "kiln:b"],
    ] {
        let (code, out, err) = answer(&s, &[&["--in", "missing", "ls"][..], operands].concat(), "");
        assert_eq!((code, out.as_str()), (2, ""), "{operands:?}: {err}");
        assert!(err.contains("--refers-to"), "{err}");
    }
    for selectors in [&["--in", "yard", "--group"][..], &["--group", "--group"]] {
        assert_eq!(
            s.fails(&[selectors, &["ls", "--refers-to", "kiln:a"]].concat(), "")
                .0,
            2
        );
    }
    let args = ["--group", "ls", "--refers-to", "kiln:a"];
    assert_eq!(
        s.fails(&args, ""),
        (2, "--group needs a local stores.json".into())
    );
    fs::write(s.dir.join("stores.json"), "{}").expect("empty group");
    assert_eq!(
        s.ok(&args),
        r#"{"counts":{"fields_read":0,"things_not_read":0,"values_not_typed":0},"fields":[],"stores":[],"target":"kiln:a","things":[]}"#
    );
    assert_eq!(
        s.ok(&[&args[..], &["--plain"]].concat()),
        "target  kiln:a\nthings  0\nstores  0\nKIND  FIELD  DESCRIPTION\nthings_not_read  0\nvalues_not_typed  0\nfields_read  0"
    );
    s.ok(&["init"]);
    s.ok(&["emit", "kiln", "--of", "kiln:east", "site=kiln:a"]);
    let report = s.json(&args[1..]);
    assert_eq!(strings(&report, &["things"]), Vec::<String>::new());
    assert_eq!(super::len(&report, &["stores"]), 1);
    assert_eq!(super::len(&report, &["fields"]), 0);
    // This was already a legal field name before the dedicated mode existed.
    assert!(
        s.ok(&["ls", "--fields", "--refers-to"])
            .contains(r#""--refers-to":{"state":"unset"}"#)
    );
}

#[test]
fn every_store_opens_before_reading_and_description_errors_have_no_partial_output() {
    let s = fixed_store("workshop-reference-open");
    let yard = fixed_store("yard-reference-open");
    let args = ["--group", "ls", "--refers-to", "kiln:east"];
    fs::write(s.dir.join(".loops/store.json"), "{").expect("invalid local description");
    fs::write(
        s.dir.join("stores.json"),
        r#"{"workshop":".","yard":"missing"}"#,
    )
    .expect("group");
    let (code, out, err) = answer(&s, &args, "");
    assert_eq!((code, out.as_str()), (1, ""));
    assert!(err.contains("store-unknown: yard"), "{err}");
    group(&s, &yard);
    let (code, out, err) = answer(&s, &args, "");
    assert_eq!((code, out.as_str()), (2, ""));
    assert!(err.contains(".loops/store.json"), "{err}");
}

#[test]
fn other_ls_modes_still_refuse_multiple_stores_before_lookup() {
    let s = Store::scratch("yard-reference-other-modes");
    for mode in [
        &["ls", "kiln:"][..],
        &["ls", "--things", "--plain"],
        &["ls", "--json", "kiln:", "--active"],
        &["ls", "--fields", "--refers-to"],
    ] {
        for (flags, message) in [
            (&["--group"][..], "--group is not supported"),
            (&["--in", "yard", "--in", "yard"], "use --in once"),
        ] {
            assert_eq!(
                answer(&s, &[flags, mode].concat(), ""),
                (2, String::new(), message.into())
            );
        }
    }
}
