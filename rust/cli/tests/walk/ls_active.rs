//! The lifecycle declared for a thing-name prefix filters its live field.
use super::{Json, Store, answer, at, fixed_store, parse_json, text};
use std::fs;

const DESCRIPTION: &str = r#"{"version":1,"kinds":{"kiln":{
    "headline":"title","preview":["title"],
    "lifecycle":{"field":"stage","closed":["retired","broken"],"active":["hot"]}
},"yard":{}}}"#;
const RULE: &str = r#"{"closed":["retired","broken"],"field":"stage","kind":"kiln"}"#;
const NEEDS: &str = "--active needs a <kind>: prefix whose kind declares lifecycle";

fn describe(s: &Store, contents: &str) {
    fs::write(s.dir.join(".loops/store.json"), contents).expect("write fixture description");
}

fn rows(result: &Json) -> &[Json] {
    let Json::Arr(rows) = at(result, &["things"]) else {
        panic!("expected things")
    };
    rows
}

#[test]
fn hide_only_a_single_live_closed_text_value_before_projection() {
    let s = fixed_store("kiln-active-states");
    describe(&s, DESCRIPTION);
    s.changeset(
        r#"{"kind":"kiln","of":"kiln:cold","payload":{"stage":"retired"}}
{"kind":"kiln","of":"kiln:missing","payload":{}}
{"kind":"kiln","of":"kiln:open","payload":{"stage":"hot"}}
{"kind":"kiln","of":"kiln:parked","payload":{"stage":"parked"}}
{"kind":"kiln","of":"kiln:other","payload":{"stage":"broken-later"}}
{"kind":"kiln","of":"kiln:list","payload":{"stage":["retired"]}}
{"kind":"kiln","of":"kiln:number","payload":{"stage":1}}
{"kind":"kiln","of":"kiln:fork","sets":["stage"],"payload":{"stage":"retired"}}
{"kind":"kiln","of":"kiln:fork","sets":["stage"],"payload":{"stage":"broken"}}
{"kind":"kiln","of":"kiln:equal","sets":["stage"],"payload":{"stage":"broken","title":"East kiln"}}
{"kind":"kiln","of":"kiln:equal","sets":["stage"],"payload":{"stage":"broken","title":"West kiln"}}
{"kind":"kiln","of":"kiln:moved","payload":{"stage":"broken"}}
{"kind":"potter","of":"kiln:changed-kind","payload":{"stage":"broken"}}"#,
    );
    // An unrelated patch inherits the closed field. Replacing it opens the kiln.
    s.ok(&["emit", "kiln", "--of", "kiln:cold", "title=Kiln"]);
    s.ok(&["emit", "kiln", "--of", "kiln:moved", "stage=hot"]);
    let erased = s.ok(&["emit", "kiln", "--of", "kiln:erased", "stage=broken"]);
    s.ok(&["erase", &erased]);
    let source = fixed_store("workshop-active-withheld");
    let withheld = source.ok(&["emit", "kiln", "--of", "kiln:withheld", "stage=retired"]);
    source.ok(&["erase", &withheld]);
    s.ok_with(&["import"], &source.ok(&["get", &withheld]));

    for projection in [&[][..], &["--fields", "title"], &["--fields", "stage"]] {
        let base = [&["ls", "kiln:"][..], projection].concat();
        let all = s.json(&base);
        let result = s.json(&[base.as_slice(), &["--active"]].concat());
        assert_eq!(at(&result, &["active"]).canon(), RULE);
        let visible = rows(&result);
        assert_eq!(
            visible
                .iter()
                .map(|r| text(r, &["thing"]))
                .collect::<Vec<_>>(),
            [
                "kiln:equal",
                "kiln:erased",
                "kiln:fork",
                "kiln:list",
                "kiln:missing",
                "kiln:moved",
                "kiln:number",
                "kiln:open",
                "kiln:other",
                "kiln:parked",
                "kiln:withheld"
            ]
        );
        let expected: Vec<_> = rows(&all)
            .iter()
            .filter(|r| !matches!(text(r, &["thing"]), "kiln:cold" | "kiln:changed-kind"))
            .collect();
        assert!(
            visible.iter().eq(expected),
            "ordinary JSON rows for {projection:?}"
        );
    }
}

#[test]
fn narrower_prefix_and_both_formats_report_exactly_what_was_applied() {
    let s = fixed_store("yard-active-prefix");
    describe(&s, DESCRIPTION);
    s.changeset(r#"{"kind":"kiln","of":"kiln:east","payload":{"stage":"hot","title":"East kiln","name":"Kiln"}}
{"kind":"kiln","of":"kiln:edge","payload":{"stage":"broken"}}
{"kind":"kiln","of":"kiln:west","payload":{"stage":"hot"}}"#);
    let usual = s.json(&["ls", "kiln:east"]);
    let expected = format!(
        r#"{{"active":{RULE},"things":{}}}"#,
        at(&usual, &["things"]).canon()
    );
    assert_eq!(s.ok(&["ls", "--active", "kiln:e", "--json"]), expected);
    for projection in [&[][..], &["--fields", "name"]] {
        // In plain mode, a preview or an explicit column can omit stage.
        let table = s.ok(&[&["ls", "kiln:east", "--plain"][..], projection].concat());
        assert_eq!(
            s.ok(&[&["ls", "kiln:e", "--plain", "--active"][..], projection].concat()),
            format!("active  {RULE}\n{table}")
        );
    }
    assert_eq!(
        s.ok(&["ls", "kiln:edge", "--active"]),
        format!(r#"{{"active":{RULE},"things":[]}}"#)
    );
    assert_eq!(
        s.ok(&["ls", "kiln:edge", "--active", "--plain"]),
        format!(
            "active  {RULE}\n{}",
            s.ok(&["ls", "kiln:nothing", "--plain"])
        )
    );
}

#[test]
fn empty_and_omitted_closed_keep_everything() {
    let s = fixed_store("kiln-active-empty");
    s.ok(&["emit", "kiln", "--of", "kiln:east", "stage=retired"]);
    let baseline = s.json(&["ls", "kiln:"]);
    for lifecycle in [
        r#"{"field":"stage","active":["hot"],"closed":[]}"#,
        r#"{"field":"stage","active":["hot"]}"#,
    ] {
        describe(
            &s,
            &format!(r#"{{"version":1,"kinds":{{"kiln":{{"lifecycle":{lifecycle}}}}}}}"#),
        );
        let result = s.json(&["ls", "kiln:", "--active"]);
        assert_eq!(rows(&result), rows(&baseline));
        assert_eq!(
            at(&result, &["active"]).canon(),
            r#"{"closed":[],"field":"stage","kind":"kiln"}"#
        );
    }
}

#[test]
fn invalid_prefix_and_description_diagnostics() {
    let s = fixed_store("yard-active-errors");
    let fail = |args: &[&str], why: &str| {
        for format in ["--json", "--plain"] {
            let args = [&["ls", format][..], args].concat();
            assert_eq!(
                answer(&s, &args, ""),
                (2, String::new(), why.to_owned()),
                "{args:?}"
            );
        }
    };
    fail(&["kiln:", "--active"], NEEDS); // No description.
    describe(&s, DESCRIPTION);
    for args in [
        &["--active"][..],
        &["--things", "--active"],
        &["--things", "kiln:", "--active"],
        &["--active", "kiln:", "--things"],
        &["kiln:", "yard:", "--active"],
        &["--active", "--fields", "title"],
        &["ki", "--active"],
        &["kiln", "--active"],
        &["potter:", "--active"],
        &["yard:", "--active"],
        &["r:", "--active"],
        &["", "--active"],
    ] {
        fail(args, NEEDS);
    }
    fail(&["kiln:", "--active", "--active"], "use --active once");
    let path = s.dir.join(".loops/store.json");
    for (lifecycle, why) in [
        ("[]", ": expected an object"),
        ("{}", ".field: missing member"),
        (r#"{"field":null}"#, ".field: expected text"),
        (r#"{"field":""}"#, ".field: expected nonempty text"),
        (
            r#"{"field":"stage"}"#,
            ": expected at least one value in closed or active",
        ),
        (
            r#"{"field":"stage","closed":[],"active":[]}"#,
            ": expected at least one value in closed or active",
        ),
        (
            r#"{"field":"stage","active":["hot","broken"],"closed":["broken"]}"#,
            ".active[1]: broken also appears in closed",
        ),
        (r#"{"closed":false}"#, ".closed: expected an array"),
        (r#"{"active":false}"#, ".active: expected an array"),
        (r#"{"closed":[1]}"#, ".closed[0]: expected text"),
        (r#"{"active":[null]}"#, ".active[0]: expected text"),
        (
            r#"{"closed":["hot","hot"]}"#,
            ".closed[1]: repeated list entry",
        ),
        (
            r#"{"active":["hot","hot"]}"#,
            ".active[1]: repeated list entry",
        ),
        (
            r#"{"field":"stage","field":"other"}"#,
            ".field: repeated member",
        ),
        (r#"{"clsoed":[]}"#, ".clsoed: unknown member"),
    ] {
        describe(
            &s,
            &format!(r#"{{"version":1,"kinds":{{"kiln":{{"lifecycle":{lifecycle}}}}}}}"#),
        );
        fail(
            &["kiln:e", "--active"],
            &format!("{}: $.kinds.kiln.lifecycle{why}", path.display()),
        );
    }
}

#[test]
fn ordinary_json_prefix_is_byte_identical_and_a_named_listing_uses_the_selected_rule() {
    let shop = fixed_store("workshop-active-origin");
    let yard = fixed_store("yard-active-selected");
    yard.ok(&["emit", "kiln", "--of", "kiln:east", "stage=hot"]);
    shop.ok(&["emit", "kiln", "--of", "kiln:west", "stage=broken"]);
    let binding = format!("yard={}", yard.dir.display());
    let flags = ["--store", &binding, "--in", "yard"];
    let invoke = |args: &[&str]| answer(&shop, &[&flags[..], args].concat(), "");
    let baseline = invoke(&["ls", "kiln:e"]);
    assert_eq!(baseline, answer(&yard, &["ls", "kiln:e"], ""));
    assert_eq!(baseline.0, 0);
    describe(&shop, "{");
    describe(&yard, DESCRIPTION);
    assert_eq!(invoke(&["ls", "kiln:e"]), baseline);
    assert_eq!(invoke(&["ls", "kiln:e", "--json"]), baseline);
    let active = invoke(&["ls", "kiln:", "--active"]);
    assert_eq!(active, answer(&yard, &["ls", "kiln:", "--active"], ""));
    assert_eq!((active.0, active.2.as_str()), (0, ""));
    let result = parse_json(&active.1).expect("named listing");
    assert_eq!(at(&result, &["active"]).canon(), RULE);
    assert_eq!(rows(&result).len(), 1);
    assert_eq!(text(&rows(&result)[0], &["thing"]), "kiln:east");

    // --fields consumes the literal field name, even when it spells the new flag.
    describe(&yard, "{");
    assert_eq!(invoke(&["ls", "kiln:e"]), baseline);
    let field = yard.json(&["ls", "kiln:e", "--fields", "--active"]);
    assert_eq!(
        text(&rows(&field)[0], &["fields", "--active", "state"]),
        "unset"
    );
}
