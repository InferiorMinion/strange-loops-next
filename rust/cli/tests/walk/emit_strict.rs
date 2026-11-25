//! The local description checks the shorthand before it signs.
use super::{Store, answer, at, fixed_emit, fixed_store, has, strings, text};
use std::fs;

const DESCRIPTION: &str = r#"{"version":1,"kinds":{
    "kiln":{"key":"name","lifecycle":{"field":"stage","active":["hot"],"closed":["cold"]},
            "refs":{"owners":["potter","yard"],"peer":[]}},
    "backups":{"key":null},"workshop":{}}}"#;

fn described(name: &str) -> Store {
    let s = fixed_store(name);
    fs::write(s.dir.join(".loops/store.json"), DESCRIPTION).expect("description");
    s
}

fn stopped(s: &Store, args: &[&str], code: i32, words: &[&str]) {
    let before = s.ok(&["export"]);
    let segments = s.segments();
    let (status, stdout, stderr) = answer(s, &[&["emit"][..], args].concat(), "");
    assert_eq!((status, stdout.as_str()), (code, ""), "{args:?}: {stderr}");
    for word in words {
        assert!(
            stderr.contains(word),
            "{args:?}: expected {word:?} in {stderr:?}"
        );
    }
    assert_eq!(s.segments(), segments, "{args:?}");
    assert_eq!(s.ok(&["export"]), before, "{args:?}");
}

#[test]
fn derived_stated_and_undescribed_writes_sign_identical_lines() {
    let derived = described("kiln-derived");
    let stated = described("kiln-stated");
    let original = fixed_store("kiln-original");
    let write = |extra: &[&str]| {
        let args = [&["kiln", "name=East kiln/a:b"][..], extra].concat();
        let explicit = [args.as_slice(), &["--of", "kiln:East kiln/a:b"]].concat();
        let id = fixed_emit(&derived, &args);
        assert_eq!(id, fixed_emit(&stated, &explicit));
        assert_eq!(id, fixed_emit(&original, &explicit));
        id
    };
    let first = write(&["--about", "yard:east", "--occurred", "2019"]);
    let revised = write(&["stage=hot"]);
    let based = write(&["stage=cold", "--base-id", &revised]);
    for (id, prior) in [(&revised, &first), (&based, &revised)] {
        let line = derived.json(&["get", id]);
        assert_eq!(text(&line, &["env", "of"]), "kiln:East kiln/a:b");
        assert!(strings(&line, &["env", "prev"]).contains(prior));
        assert_eq!(strings(&line, &["env", "sets"]), ["name", "stage"]);
        assert_eq!(strings(&line, &["env", "about"]), ["yard:east"]);
        assert_eq!(text(&line, &["env", "occurred"]), "2019");
    }
    assert_eq!(derived.ok(&["export"]), original.ok(&["export"]));
    assert_eq!(stated.ok(&["export"]), original.ok(&["export"]));
}

#[test]
fn key_checks_use_the_resolved_thing_and_keep_writer_refusals() {
    let s = described("kiln-resolution");
    stopped(
        &s,
        &["kiln", "stage=hot"],
        2,
        &["kiln", "name=<key>", "--of", "--root"],
    );
    let first = s.ok(&["emit", "kiln", "--of", "yard:east:1", "stage=hot"]);
    for args in [
        vec!["--of", "yard:east:1", "name=east:1"],
        vec!["--prev", &first, "stage=cold"],
        vec!["--prev", &first, "name=east:1"],
        vec!["--prev", &first, "--of", "yard:east:1"],
        vec!["--prev", &first, "--of", "yard:east:1", "name=east:1"],
    ] {
        let id = s.ok(&[&["emit", "kiln"][..], &args].concat());
        assert_eq!(text(&s.json(&["get", &id]), &["env", "of"]), "yard:east:1");
    }
    for (args, source) in [
        (vec!["--of", "yard:east:1"], "--of"),
        (vec!["--prev", &first], "--prev"),
        (vec!["--prev", &first, "--of", "yard:east:1"], "--of"),
    ] {
        stopped(
            &s,
            &[&["kiln"][..], &args, &["name=East:1"]].concat(),
            2,
            &[
                "name=East:1",
                "east:1",
                "yard:east:1",
                source,
                "agree",
                "emit -",
            ],
        );
    }
    for flags in [&[][..], &["--of", "yard:east:1"], &["--prev", &first]] {
        for (field, why) in [
            ("name=", "nonempty text"),
            ("name:=7", "nonempty text"),
            ("name=e,1", "comma"),
        ] {
            stopped(
                &s,
                &[&["kiln"][..], flags, &[field]].concat(),
                2,
                &["name", why, "emit -"],
            );
        }
    }
    let other = s.ok(&["emit", "kiln", "name=west"]);
    let unknown = format!("r:{}", "0".repeat(52));
    for (args, reason) in [
        (
            vec!["--of", "kiln:west", "--prev", &first, "name=west"],
            "prev-unrelated",
        ),
        (vec!["--prev", &unknown], "prev-unknown"),
        (vec!["--prev", &first, "--prev", &other], "prev-unrelated"),
        (vec!["--prev", "zzzz"], "id-unknown"),
        (vec!["--of", "Yard:east", "name=west"], "of-not-typed"),
        (vec!["--prev", "@yard", "name="], "label-unknown"),
        (vec!["name=", "name=west"], "member-duplicate"),
    ] {
        stopped(
            &s,
            &[&["kiln"][..], &args].concat(),
            1,
            &["refused:", reason],
        );
    }
}

#[test]
fn kind_and_fields_still_apply_to_roots_and_revisions_of_roots() {
    let s = described("kiln-root-rules");
    let root = s.ok(&["emit", "kiln", "--root", "name=east"]);
    let line = s.json(&["get", &root]);
    assert!(!has(&line, &["env"], "of"));
    assert_eq!(text(&line, &["payload", "name"]), "east");
    let next = s.ok(&["emit", "kiln", "--prev", &root, "name=west"]);
    for flags in [
        &["--root"][..],
        &["--of", &root],
        &["--prev", &root],
        &["--prev", &next],
    ] {
        for field in ["name:=7", "name=", "name=e,w"] {
            let id = s.ok(&[&["emit", "kiln"][..], flags, &[field]].concat());
            let line = s.json(&["get", &id]);
            if flags == ["--root"] {
                assert!(!has(&line, &["env"], "of"));
            } else {
                assert_eq!(text(&line, &["env", "of"]), root);
            }
            let stored = at(&line, &["payload", "name"]).canon();
            assert_eq!(
                stored,
                match field {
                    "name:=7" => "7",
                    "name=" => "\"\"",
                    _ => "\"e,w\"",
                }
            );
        }
        stopped(
            &s,
            &[&["potter"][..], flags].concat(),
            2,
            &["potter", "[\"backups\",\"kiln\",\"workshop\"]", "emit -"],
        );
        stopped(
            &s,
            &[&["kiln"][..], flags, &["stage=warm"]].concat(),
            2,
            &["stage=warm", "hot", "cold", "emit -"],
        );
        stopped(
            &s,
            &[&["kiln"][..], flags, &["owners=workshop:east"]].concat(),
            2,
            &["owners", "workshop:east", "potter", "yard", "emit -"],
        );
    }
    stopped(&s, &["potter"], 2, &["potter", "kiln", "emit -"]);
    stopped(
        &s,
        &["kiln", "--root", "--of", "kiln:east"],
        2,
        &["--root", "--of", "--prev"],
    );
    stopped(
        &s,
        &["kiln", "--prev", &next, "--root"],
        2,
        &["--root", "--of", "--prev"],
    );
    let id = s.ok(&["emit", "kiln", "--root"]);
    assert!(!has(&s.json(&["get", &id]), &["env"], "of"));
}

#[test]
fn null_and_omitted_keys_keep_the_original_shorthand() {
    let s = described("workshop-no-key");
    let original = fixed_store("workshop-no-description");
    for kind in ["backups", "workshop"] {
        for args in [
            vec![kind],
            vec![kind, "name=east"],
            vec![kind, "--of", "yard:east"],
        ] {
            assert_eq!(fixed_emit(&s, &args), fixed_emit(&original, &args));
        }
    }
    let root = fixed_emit(&original, &["kiln", "--root", "name=east"]);
    assert_eq!(root, fixed_emit(&original, &["kiln", "name=east"]));
    for flags in [&["--of", "yard:east"][..], &["--prev", &root]] {
        stopped(
            &original,
            &[&["kiln", "--root"][..], flags].concat(),
            2,
            &["--root", "--of", "--prev"],
        );
    }
}

#[test]
fn reference_forms_lifecycle_and_commas_are_checked_only_as_declared() {
    let s = described("yard-field-rules");
    let root = s.ok(&["emit", "workshop"]);
    let args = ["kiln", "--of", "yard:e,w"];
    for fields in [
        vec!["stage=hot", "owners=potter:missing"],
        vec!["stage=cold", r#"owners:=["potter:a","yard:b"]"#],
        vec!["owners:=[]", "--about", "yard:w,e", "extra=w,e"],
        vec!["--sets", "stage", "--sets", "owners"],
    ] {
        s.ok(&[&["emit"][..], &args, &fields].concat());
    }
    s.ok(&[
        "emit",
        "kiln",
        "name=east",
        &format!("peer={root}"),
        "owners:=[]",
    ]);
    for (field, typed) in [
        ("owners:=null", "null"),
        ("owners:=7", "7"),
        ("owners:={}", "{}"),
        (r#"owners:=[["yard:a"]]"#, r#"["yard:a"]"#),
        (r#"owners:=["yard:a",false]"#, "false"),
        ("owners=potter", "potter"),
        ("owners=potter:", "potter:"),
        ("owners=kiln:a", "kiln:a"),
        (&format!("owners={root}"), root.as_str()),
    ] {
        stopped(
            &s,
            &[&args[..], &[field]].concat(),
            2,
            &["owners", typed, "[\"potter\",\"yard\"]", "emit -"],
        );
    }
    for field in [
        "owners=potter:a,potter:b",
        r#"owners:=["yard:a","potter:a,potter:b"]"#,
    ] {
        stopped(
            &s,
            &[&args[..], &[field]].concat(),
            2,
            &[
                "potter:a,potter:b",
                "comma",
                r#"owners:='["potter:a","potter:b"]'"#,
                "emit -",
            ],
        );
    }
    stopped(
        &s,
        &[&args[..], &["peer=potter"]].concat(),
        2,
        &["potter", "any prefix", "emit -"],
    );
    for field in ["stage:=null", "stage:=2", r#"stage:=["hot"]"#] {
        stopped(
            &s,
            &[&args[..], &[field]].concat(),
            2,
            &["stage", "hot", "cold", "text", "emit -"],
        );
    }
}

#[test]
fn named_emits_validate_the_destination_description() {
    let origin = fixed_store("workshop-emit-origin");
    let yard = described("yard-emit-destination");
    let bind = format!("yard={}", yard.dir.display());
    let prefix = ["--store", &bind, "--in", "yard", "emit", "kiln"];
    fs::write(origin.dir.join(".loops/store.json"), "{").expect("broken origin");
    let id = origin.ok(&[&prefix[..], &["name=east"]].concat());
    assert_eq!(text(&yard.json(&["get", &id]), &["env", "of"]), "kiln:east");
    assert_eq!(origin.ok(&["export"]), "");
    let before = yard.ok(&["export"]);
    let reject = |field: &str, word: &str| {
        let (code, out, err) = answer(&origin, &[&prefix[..], &[field]].concat(), "");
        assert_eq!((code, out.as_str()), (2, ""));
        assert!(err.contains(word), "{err}");
        assert_eq!(yard.ok(&["export"]), before);
    };
    reject("stage=hot", "--root");
    fs::write(yard.dir.join(".loops/store.json"), "{").expect("broken destination");
    reject("name=east", "store.json: $");
}
