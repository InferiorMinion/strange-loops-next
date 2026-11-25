//! Edit the actual heads' copied mentions on one shorthand emit.
use super::{Store, answer, fixed_emit, fixed_store, has, strings};
use std::fs;

const THING: &[&str] = &["kiln", "--of", "kiln:east"];
const SPEC: &str = r#"{"vertex":"kiln","from":[{"store":"."}],"fold":{"kilns":{"count":true}}}"#;

fn emit(s: &Store, flags: &[&str]) -> String {
    fixed_emit(s, &[THING, flags].concat())
}

fn about(s: &Store, id: &str) -> Vec<String> {
    let line = s.json(&["get", id]);
    if has(&line, &["env"], "about") {
        strings(&line, &["env", "about"])
    } else {
        Vec::new()
    }
}

#[test]
fn edit_the_union_of_both_heads_and_carry_it_forward() {
    let s = fixed_store("kiln-about-heads");
    s.changeset(
        r#"{"kind":"kiln","of":"kiln:east","about":["yard:a","potter:p"]}
{"kind":"kiln","of":"kiln:east","about":["yard:b","potter:p"]}"#,
    );
    let copy = fixed_store("kiln-about-copy");
    copy.ok_with(&["import"], &s.ok(&["export"]));
    assert_eq!(
        about(&copy, &emit(&copy, &[])),
        ["potter:p", "yard:a", "yard:b"]
    );

    let id = emit(&s, &["--drop-about", "yard:a", "--add-about", "kiln:c"]);
    let expected = ["kiln:c", "potter:p", "yard:b"];
    assert_eq!(about(&s, &id), expected);
    assert_eq!(about(&s, &emit(&s, &[])), expected);
}

#[test]
fn edits_and_whole_sets_produce_identical_signed_lines() {
    let edited = fixed_store("yard-about-edited");
    let explicit = fixed_store("yard-about-explicit");
    for s in [&edited, &explicit] {
        emit(s, &["--about", "potter:p"]);
    }
    for (edits, whole, expected) in [
        (
            &["--add-about", "yard:east"][..],
            &["--about", "yard:east", "--about", "potter:p"][..],
            &["potter:p", "yard:east"][..],
        ),
        (
            &["--drop-about", "potter:p"],
            &["--about", "yard:east"],
            &["yard:east"],
        ),
        (
            &[
                "--add-about",
                "yard:east",
                "--add-about",
                "yard:east",
                "--drop-about",
                "potter:p",
                "--drop-about",
                "potter:p",
            ],
            &[],
            &["yard:east"],
        ),
        (&["--drop-about", "yard:east"], &["--no-about"], &[]),
    ] {
        let id = emit(&edited, edits);
        assert_eq!(id, emit(&explicit, whole));
        assert_eq!(about(&edited, &id), expected);
        assert_eq!(edited.ok(&["get", &id]), explicit.ok(&["get", &id]));
    }
    assert_eq!(
        answer(&edited, &["export"], ""),
        answer(&explicit, &["export"], "")
    );
}

#[test]
fn based_edit_uses_the_heads_in_the_cover() {
    let s = fixed_store("workshop-about-base");
    let old = emit(&s, &["--about", "potter:p", "--about", "yard:old"]);
    let base = s.ok_with(&["tick", "-", "--all"], SPEC);
    emit(&s, &["--about", "yard:new"]);
    let id = emit(
        &s,
        &[
            "--base",
            &base,
            "--drop-about",
            "potter:p",
            "--add-about",
            "kiln:c",
        ],
    );
    assert_eq!(strings(&s.json(&["get", &id]), &["env", "prev"]), [old]);
    assert_eq!(about(&s, &id), ["kiln:c", "yard:old"]);
}

#[test]
fn literal_names_and_reference_fields_stay_separate() {
    let s = fixed_store("potter-about-literals");
    fs::write(
        s.dir.join(".loops/store.json"),
        r#"{"version":1,"kinds":{"kiln":{"refs":{"owner":["potter"]}}}}"#,
    )
    .expect("description");
    let root = fixed_emit(
        &s,
        &[
            "kiln",
            "--root",
            "--drop-about",
            "yard:a",
            "--add-about",
            "yard:a,b",
            "owner=potter:p",
        ],
    );
    assert_eq!(about(&s, &root), ["yard:a,b"]);
    let short = &root[..10];
    let first = emit(
        &s,
        &[
            "--add-about",
            short,
            "--add-about",
            "yard:\u{e000}",
            "--add-about",
            "yard:\u{10000}",
            "owner=potter:p",
        ],
    );
    assert_eq!(
        about(&s, &first),
        [short, "yard:\u{10000}", "yard:\u{e000}"]
    );
    let id = emit(&s, &["--drop-about", &root]);
    assert_eq!(about(&s, &id), about(&s, &first));
    rejected(
        &s,
        &["emit", "kiln", "--add-about", "yard:a", "owner=yard:a"],
        "",
        2,
        &["owner", "potter"],
    );
}

fn rejected(s: &Store, args: &[&str], input: &str, code: i32, words: &[&str]) {
    let before = s.ok(&["export"]);
    let segments = s.segments();
    let (status, out, err) = answer(s, args, input);
    assert_eq!((status, out.as_str()), (code, ""), "{args:?}: {err}");
    for word in words {
        assert!(err.contains(word), "{args:?}: {err}");
    }
    assert_eq!(s.segments(), segments);
    assert_eq!(s.ok(&["export"]), before);
}

#[test]
fn conflicting_options_and_invalid_operands_leave_no_record() {
    let s = fixed_store("kiln-about-errors");
    emit(&s, &["--about", "potter:p"]);
    for flag in ["--add-about", "--drop-about"] {
        for whole in [&["--about", "yard:a"][..], &["--no-about"]] {
            for pair in [[whole, &[flag, "yard:b"]], [&[flag, "yard:b"], whole]] {
                rejected(
                    &s,
                    &[&["emit"][..], THING, pair[0], pair[1]].concat(),
                    "",
                    2,
                    &[whole[0], flag],
                );
            }
        }
        for (invalid, refusal) in [("yard", "about-not-typed"), ("@yard", "label-unknown")] {
            rejected(
                &s,
                &[&["emit"][..], THING, &[flag, invalid]].concat(),
                "",
                1,
                &[refusal],
            );
        }
        rejected(
            &s,
            &[&["emit"][..], THING, &[flag]].concat(),
            "",
            2,
            &[flag, "value"],
        );
        for args in [
            vec!["tick", "-", flag, "yard:a"],
            vec!["emit", "-", flag, "yard:a"],
            vec!["emit", "-", "--fill", flag, "yard:a"],
        ] {
            rejected(&s, &args, SPEC, 2, &[flag]);
        }
    }
    for (first, second) in [
        ("--add-about", "--drop-about"),
        ("--drop-about", "--add-about"),
    ] {
        rejected(
            &s,
            &[&["emit"][..], THING, &[first, "yard:a", second, "yard:a"]].concat(),
            "",
            2,
            &[first, second, "yard:a"],
        );
    }
    rejected(
        &s,
        &["emit", "kiln", "--about", "yard:a", "--no-about"],
        "",
        2,
        &["--about", "--no-about"],
    );
}
