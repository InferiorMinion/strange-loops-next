//! The product walk: one person emits, reads and folds through the `loops` binary.
use loops::json::Json;
use loops_eval::parse_json;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

struct Store {
    dir: PathBuf,
}
impl Store {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("walk-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch directory");
        let store = Self { dir };
        store.ok(&["init"]);
        store
    }
    fn run(&self, args: &[&str], stdin: &str) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_loops"))
            .arg("-C")
            .arg(&self.dir)
            .args(args)
            .env_remove("LOOPS_KEY")
            .env_remove("LOOPS_OBSERVER")
            .env("LOOPS_KEY_FILE", self.dir.join("device-key"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("loops runs");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(stdin.as_bytes())
            .expect("stdin written");
        child.wait_with_output().expect("loops exits")
    }
    fn ok_with(&self, args: &[&str], stdin: &str) -> String {
        let out = self.run(args, stdin);
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout)
            .expect("UTF-8")
            .trim_end()
            .to_owned()
    }
    fn ok(&self, args: &[&str]) -> String {
        self.ok_with(args, "")
    }
    fn json(&self, args: &[&str]) -> Json {
        parse_json(&self.ok(args)).expect("JSON answer")
    }
    /// The exit code and what stderr says.
    fn fails(&self, args: &[&str], stdin: &str) -> (i32, String) {
        let out = self.run(args, stdin);
        (
            out.status.code().expect("exit code"),
            String::from_utf8_lossy(&out.stderr).trim_end().to_owned(),
        )
    }
    fn segments(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        for dir in fs::read_dir(self.dir.join(".loops/segments")).expect("segments") {
            for file in fs::read_dir(dir.expect("entry").path()).expect("key directory") {
                found.push(file.expect("entry").path());
            }
        }
        found.sort();
        found
    }
}
fn at<'a>(j: &'a Json, path: &[&str]) -> &'a Json {
    path.iter().fold(j, |j, name| match j {
        Json::Obj(members) => members
            .iter()
            .find(|(k, _)| k == name)
            .map_or_else(|| panic!("no member {name}"), |(_, v)| v),
        _ => panic!("{name}: not an object"),
    })
}
fn text<'a>(j: &'a Json, path: &[&str]) -> &'a str {
    match at(j, path) {
        Json::Str(s) => s,
        other => panic!("{path:?}: {other:?}"),
    }
}
fn len(j: &Json, path: &[&str]) -> usize {
    match at(j, path) {
        Json::Arr(items) => items.len(),
        other => panic!("{path:?}: {other:?}"),
    }
}
const SPEC: &str = r#"{"vertex":"decisions","from":[{"store":".","kind":["decision"]}],"fold":{
  "all":{"count":true},
  "settled":{"count":true,"where":{"status":{"is":"settled"}}},
  "weight":{"sum":"n"},
  "current":{"latest-by":"of","follow":"prev"}}}"#;

const FOLLOW: &str = r#"{"vertex":"now","from":[{"store":".","kind":["task"]}],"fold":{
  "all":{"count":true},
  "now":{"count":true,"follow":"prev"},
  "open":{"count":true,"follow":"prev","where":{"status":{"is":"open"}}},
  "est":{"sum":"n","follow":"prev"}}}"#;

/// A slot's value and unread count.
fn tally(slots: &Json, slot: &str) -> (Json, Json) {
    (
        at(slots, &[slot, "value"]).clone(),
        at(slots, &[slot, "unread"]).clone(),
    )
}

#[test]
fn count_and_sum_follow_the_heads() {
    let s = Store::new("now");
    let n = |v: i64, u: i64| (Json::Int(v.into()), Json::Int(u.into()));
    let first = s.ok(&[
        "emit",
        "task",
        "--of",
        "task:1",
        "status=open",
        "n:=3",
        "Open",
    ]);
    // Two revisions of the first: a fork, and both heads are current.
    s.ok(&[
        "emit",
        "task",
        "--prev",
        &first,
        "status=open",
        "n:=5",
        "Bigger",
    ]);
    let fork = s.ok(&[
        "emit",
        "task",
        "--prev",
        &first,
        "status=done",
        "n:=1",
        "Done",
    ]);
    let slots = parse_json(&s.ok_with(&["fold", "-"], FOLLOW)).expect("JSON");
    let got = ["all", "now", "open", "est"].map(|k| tally(&slots, k));
    assert_eq!(got, [n(3, 0), n(2, 0), n(1, 0), n(6, 0)]);
    // An erased head is still current: counted by its envelope, unread to a sum.
    s.json(&["erase", &fork[2..12], "not ours to say"]);
    let slots = parse_json(&s.ok_with(&["fold", "-"], FOLLOW)).expect("JSON");
    let got = ["all", "now", "open", "est"].map(|k| tally(&slots, k));
    assert_eq!(got, [n(3, 0), n(2, 0), n(1, 1), n(5, 1)]);
}

const GROUPED: &str = r#"{"vertex":"per-project","from":[{"store":".","kind":["task"]}],"fold":{
  "every":{"count":true,"by":"env.about:project"},
  "now":{"count":true,"follow":"prev","by":"env.about:project"}}}"#;

#[test]
fn count_groups_by_mention_over_every_record_and_over_the_heads() {
    let s = Store::new("per-project");
    let first = s.ok(&[
        "emit",
        "task",
        "--of",
        "task:1",
        "--about",
        "project:x",
        "Open",
    ]);
    // The task moves to project y: its revision mentions y, not x.
    s.ok(&[
        "emit",
        "task",
        "--prev",
        &first,
        "--about",
        "project:y",
        "Moved",
    ]);
    s.ok(&[
        "emit",
        "task",
        "--of",
        "task:2",
        "--about",
        "project:x",
        "Other",
    ]);
    let slots = parse_json(&s.ok_with(&["fold", "-"], GROUPED)).expect("JSON");
    let count = |slot: &str, key: &str| at(&slots, &[slot, "groups", key, "value"]).clone();
    let n = |v: i64| Json::Int(v.into());
    // Every record: the moved task's first version still counts in x.
    assert_eq!([count("every", "x"), count("every", "y")], [n(2), n(1)]);
    // Over the heads, only the current version counts, in y.
    assert_eq!([count("now", "x"), count("now", "y")], [n(1), n(1)]);
    for slot in ["every", "now"] {
        assert_eq!(at(&slots, &[slot, "unread"]), &n(0));
        assert_eq!(at(&slots, &[slot, "ungrouped"]), &n(0));
        let Json::Obj(groups) = at(&slots, &[slot, "groups"]) else {
            panic!("groups")
        };
        let keys: Vec<&str> = groups.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["x", "y"]);
    }
}

#[test]
fn emit_revise_fork_fold_and_erase() {
    let s = Store::new("decision");
    let thing = "repo:strange-loops";
    let first = s.ok(&[
        "emit",
        "decision",
        "--of",
        thing,
        "status=leaning",
        "n:=1",
        "Go, in the old repo",
    ]);
    assert!(first.starts_with("r:") && first.len() == 54);
    // The first emit made the signing key, and the record names it.
    let who = s.json(&["whoami"]);
    assert_eq!(text(&who, &["key"]), text(&who, &["observer"]));
    let got = s.json(&["get", &first[2..10]]);
    assert_eq!(text(&got, &["env", "observer"]), text(&who, &["key"]));
    assert_eq!(text(&got, &["payload", "text"]), "Go, in the old repo");
    assert_eq!(text(&got, &["payload", "status"]), "leaning");

    // A revision replaces it; a second revision of the same record is a fork, and both heads show.
    // Written without --of, a revision is of what it replaces, and its envelope says so.
    let second = s.ok(&[
        "emit",
        "decision",
        "--about",
        "lang:rust",
        "--prev",
        &first[2..10],
        "status=settled",
        "n:=2",
        "Rust, in a new repo",
    ]);
    let got = s.json(&["get", &second]);
    assert_eq!(text(&got, &["env", "of"]), thing);
    assert_eq!(
        at(&got, &["env", "about"]),
        &Json::Arr(vec![Json::Str("lang:rust".into())])
    );
    let state = s.json(&["of", thing]);
    assert_eq!(text(&state, &["winner", "id"]), second);
    assert_eq!(len(&state, &["others"]), 0);
    // A mention makes no head.
    let mentioned = s.json(&["of", "lang:rust"]);
    assert_eq!(at(&mentioned, &["winner"]), &Json::Null);
    assert_eq!(at(&mentioned, &["records"]), &Json::Int(0_i64.into()));
    // The verb that shows a thing is `of`; `about` names no verb.
    assert_eq!(s.fails(&["about", thing], "").0, 2);
    let fork = s.ok(&[
        "emit",
        "decision",
        "--of",
        thing,
        "--prev",
        &first,
        "status=settled",
        "n:=4",
        "Zig",
    ]);
    let state = s.json(&["of", thing]);
    assert_eq!(text(&state, &["winner", "id"]), fork);
    assert_eq!(len(&state, &["others"]), 1);
    assert_eq!(at(&state, &["records"]), &Json::Int(3_i64.into()));

    // A fold reads the same set: counts cover history, latest-by the heads.
    fs::create_dir_all(s.dir.join(".loops/vertices")).expect("vertices");
    fs::write(s.dir.join(".loops/vertices/decisions.json"), SPEC).expect("spec");
    let slots = s.json(&["read", "decisions"]);
    assert_eq!(
        slots,
        parse_json(&s.ok_with(&["fold", "-"], SPEC)).expect("JSON")
    );
    assert_eq!(at(&slots, &["all", "value"]), &Json::Int(3_i64.into()));
    assert_eq!(at(&slots, &["settled", "value"]), &Json::Int(2_i64.into()));
    assert_eq!(at(&slots, &["weight", "value"]), &Json::Int(7_i64.into()));
    assert_eq!(len(&slots, &["current", "things", thing]), 2);

    // Erasing drops the opening here and says so to every reader: the id, the
    // signature and the fork stay, and the folds count what they can't read.
    let before = s.ok(&["export"]);
    let erasure = s.json(&["erase", &fork[2..12], "not ours to say"]);
    assert_eq!(
        at(&erasure, &["openings-removed"]),
        &Json::Int(1_i64.into())
    );
    let gone = s.json(&["get", &fork]);
    assert_eq!(text(&gone, &["id"]), fork);
    assert!(text(&gone, &["commit"]).starts_with("h:"));
    assert!(!s.ok(&["export"]).contains("Zig") && before.contains("Zig"));
    for file in s.segments() {
        assert!(!fs::read_to_string(file).expect("segment").contains("Zig"));
    }
    let slots = s.json(&["read", "decisions"]);
    assert_eq!(at(&slots, &["all", "value"]), &Json::Int(3_i64.into()));
    assert_eq!(at(&slots, &["weight", "value"]), &Json::Int(3_i64.into()));
    assert_eq!(at(&slots, &["weight", "unread"]), &Json::Int(1_i64.into()));
    assert_eq!(at(&slots, &["settled", "unread"]), &Json::Int(1_i64.into()));
    assert_eq!(text(&s.json(&["of", thing]), &["winner", "id"]), fork);
    let check = s.json(&["check"]);
    assert_eq!(
        (len(&check, &["refused"]), len(&check, &["altered"])),
        (0, 0)
    );
}

#[test]
fn a_batch_is_one_changeset_and_labels_name_its_records() {
    let s = Store::new("batch");
    let drafts = r#"{"label":"@d","kind":"decision","of":"repo:strange-loops","payload":{"text":"Rust, in a new repo"}}
{"kind":"note","about":["@d"],"payload":{"text":"phones and extensions need the core itself"}}
{"label":"@d2","kind":"decision","of":"repo:strange-loops","prev":["@d"],"payload":{"text":"until the rename"}}
"#;
    let out = parse_json(&s.ok_with(&["emit", "-"], drafts)).expect("JSON");
    let Json::Arr(records) = at(&out, &["records"]) else {
        panic!("records")
    };
    assert_eq!(records.len(), 3);
    assert_eq!(text(&records[0], &["label"]), "@d");
    assert_eq!(s.segments().len(), 1, "one changeset, one segment");
    let note = s.json(&["get", text(&records[1], &["id"])]);
    assert_eq!(
        at(&note, &["env", "about"]),
        &Json::Arr(vec![Json::Str(text(&records[0], &["id"]).into())])
    );
    let state = s.json(&["of", "repo:strange-loops"]);
    assert_eq!(text(&state, &["winner", "id"]), text(&records[2], &["id"]));
    // The note mentions the first decision, which is of the repo: neither the
    // mention nor the decision's id makes a thing with records of it.
    let mentioned = s.json(&["of", text(&records[0], &["id"])]);
    assert_eq!(at(&mentioned, &["records"]), &Json::Int(0_i64.into()));
    assert_eq!(at(&mentioned, &["winner"]), &Json::Null);
    assert_eq!(
        s.ok(&["export", "--about", text(&records[0], &["id"])]),
        s.ok(&["export", "--kind", "note"])
    );
    assert_eq!(s.ok(&["export", "--of", text(&records[0], &["id"])]), "");
    assert_eq!(
        s.ok(&["export", "--of", "repo:strange-loops"])
            .lines()
            .count(),
        2
    );

    // A label in `of` names a record of the same changeset.
    let out = parse_json(&s.ok_with(
        &["emit", "-"],
        "{\"label\":\"@r\",\"kind\":\"note\"}\n{\"kind\":\"note\",\"of\":\"@r\",\"prev\":[\"@r\"]}\n",
    ))
    .expect("JSON");
    let Json::Arr(pair) = at(&out, &["records"]) else {
        panic!("records")
    };
    assert_eq!(
        text(&s.json(&["get", text(&pair[1], &["id"])]), &["env", "of"]),
        text(&pair[0], &["id"])
    );

    // A label in `under` names a record of the same changeset: a grant and a
    // record acting under it, written together.
    let out = parse_json(&s.ok_with(
        &["emit", "-"],
        "{\"kind\":\"note\",\"under\":\"@g\"}\n{\"label\":\"@g\",\"kind\":\"grant\"}\n",
    ))
    .expect("JSON");
    let Json::Arr(acting) = at(&out, &["records"]) else {
        panic!("records")
    };
    assert_eq!(
        text(
            &s.json(&["get", text(&acting[0], &["id"])]),
            &["env", "under"]
        ),
        text(&acting[1], &["id"])
    );

    // Nothing is written unless every draft is taken.
    let held = s.ok(&["export"]);
    for (bad, code) in [
        (r#"{"kind":"note","about":["@nobody"]}"#, "label-unknown"),
        (r#"{"kind":"note","of":"@nobody"}"#, "label-unknown"),
        (r#"{"kind":"note","under":"@nobody"}"#, "label-unknown"),
        (
            r#"{"label":"@u","kind":"grant","under":"@u"}"#,
            "label-cycle",
        ),
        (r#"{"kind":"","payload":{}}"#, "kind-empty"),
        (
            r#"{"kind":"note","of":"thing:x","prev":["r:0000000000000000000000000000000000000000000000000000"]}"#,
            "prev-unknown",
        ),
        // A changeset states its `of`: nothing fills it in.
        (
            r#"{"kind":"note","prev":["r:0000000000000000000000000000000000000000000000000000"]}"#,
            "of-missing",
        ),
        (
            "{\"label\":\"@a\",\"kind\":\"note\",\"of\":\"thing:x\"}\n{\"kind\":\"note\",\"of\":\"@a\"}",
            "of-not-root",
        ),
        (
            r#"{"kind":"note","colour":"red"}"#,
            "draft-field-unknown at line 2: colour",
        ),
        (
            r#"{"kind":"note","payload":{"n":9007199254740993}}"#,
            "number-out-of-range",
        ),
    ] {
        let (exit, said) = s.fails(&["emit", "-"], &format!("{{\"kind\":\"note\"}}\n{bad}\n"));
        assert_eq!(
            (exit, said.as_str()),
            (1, format!("refused: {code}").as_str())
        );
    }
    assert_eq!(s.ok(&["export"]), held);
    assert_eq!(s.segments().len(), 3);
}

#[test]
fn a_changeset_line_gets_the_cores_code_for_a_missing_or_mistyped_field() {
    let s = Store::new("mistyped");
    s.ok_with(&["emit", "-"], "{\"kind\":\"note\"}\n");
    let held = s.ok(&["export"]);
    for (bad, code) in [
        // A missing or mistyped envelope field gets the core's code for it,
        // in the writer's order across the changeset (Kyle, 2026-11-11).
        (r#"{"kind":7}"#, "kind-empty"),
        (r#"{"payload":{}}"#, "kind-empty"),
        (r#"{"kind":"note","observer":7}"#, "observer-not-typed"),
        (r#"{"kind":"note","emitted":null}"#, "emitted-not-rfc3339"),
        (r#"{"kind":"note","under":["@x"]}"#, "under-not-record-id"),
        // A label beside a mistyped member is still a label, checked first.
        (r#"{"kind":"note","about":["@nobody",7]}"#, "label-unknown"),
        (
            "{\"label\":\"@a\",\"kind\":\"note\"}\n{\"kind\":\"note\",\"about\":[\"@a\",7]}",
            "about-not-typed",
        ),
        // A label refusal on one line outranks a mistyped field on another.
        (
            "{\"kind\":7}\n{\"kind\":\"note\",\"about\":[\"@nobody\"]}",
            "label-unknown",
        ),
        // A salt that is not base32hex text is not 16 bytes, after the envelope.
        (r#"{"kind":"note","salt":7}"#, "salt-not-16-bytes"),
        (r#"{"kind":"note","salt":"!"}"#, "salt-not-16-bytes"),
        ("{\"kind\":\"note\",\"salt\":7}\n{\"kind\":7}", "kind-empty"),
        // The line's own shape stays the CLI's.
        ("7", "draft-shape at line 2: a draft is a JSON object"),
        (
            r#"{"label":7,"kind":"note"}"#,
            "draft-shape at line 2: label is text",
        ),
    ] {
        let (exit, said) = s.fails(&["emit", "-"], &format!("{{\"kind\":\"note\"}}\n{bad}\n"));
        assert_eq!(
            (exit, said.as_str()),
            (1, format!("refused: {code}").as_str()),
            "{bad}"
        );
    }
    assert_eq!(s.ok(&["export"]), held, "nothing is written");
}

#[test]
fn a_revision_is_of_what_it_replaces_and_the_store_decides_of_not_root() {
    let s = Store::new("of");
    let decision = s.ok(&["emit", "decision", "--of", "repo:strange-loops", "start"]);
    let decision = decision.as_str();
    // A revision written without --of is of what it replaces: here a root,
    // whose thing is its own id, and then a revision of it, whose thing is its `of`.
    let root = s.ok(&["emit", "note", "first"]);
    let revised = s.ok(&["emit", "note", "--prev", &root, "second"]);
    let again = s.ok(&["emit", "note", "--prev", &revised, "third"]);
    for id in [&revised, &again] {
        assert_eq!(text(&s.json(&["get", id]), &["env", "of"]), root);
    }
    let state = s.json(&["of", &root]);
    assert_eq!(text(&state, &["winner", "id"]), again);
    assert_eq!(len(&state, &["others"]), 0);
    assert_eq!(at(&state, &["records"]), &Json::Int(3_i64.into()));

    // What the writer holds decides `of-not-root` and `prev-unrelated`, from the store.
    let held = s.ok(&["export"]);
    for (args, code) in [
        (vec!["--of", revised.as_str()], "of-not-root"),
        (vec!["--of", decision], "of-not-root"),
        (
            vec!["--of", &revised[2..12], "--prev", &again],
            "of-not-root",
        ),
        (vec!["--prev", &root, "--prev", decision], "prev-unrelated"),
        (
            vec!["--of", "repo:other", "--prev", &again],
            "prev-unrelated",
        ),
        (vec!["--of", "Untyped:x"], "of-not-typed"),
    ] {
        let mut argv = vec!["emit", "note"];
        argv.extend(args);
        assert_eq!(
            s.fails(&argv, ""),
            (1, format!("refused: {code}")),
            "{argv:?}"
        );
    }
    assert_eq!(s.ok(&["export"]), held);
    // An `of` naming a root is a revision's thing, stated rather than filled in.
    let stated = s.ok(&["emit", "note", "--of", &root, "--prev", &again, "fourth"]);
    assert_eq!(text(&s.json(&["of", &root]), &["winner", "id"]), stated);
}

#[test]
fn two_stores_converge_by_export_and_import() {
    let (a, b) = (Store::new("sync-a"), Store::new("sync-b"));
    let thing = "repo:strange-loops";
    let root = a.ok(&["emit", "decision", "--of", thing, "start"]);
    b.ok_with(&["import"], &a.ok(&["export"]));
    // Each revises what both hold, apart; a third record exists only in b.
    // A revision is of what it replaces, filled in from the imported line.
    let from_a = a.ok(&["emit", "decision", "--prev", &root, "a's revision"]);
    let from_b = b.ok(&["emit", "decision", "--prev", &root, "b's revision"]);
    b.ok(&["emit", "note", "--of", thing, "only b so far"]);
    // A record that only mentions the thing is none of its heads.
    b.ok(&["emit", "note", "--about", thing, "said in passing"]);
    assert_ne!(a.ok(&["export"]), b.ok(&["export"]));
    // b can't revise a record it doesn't hold.
    assert_eq!(
        b.fails(&["emit", "note", "--prev", &from_a, "x"], ""),
        (1, "refused: prev-unknown".into())
    );
    b.ok_with(&["import"], &a.ok(&["export"]));
    a.ok_with(&["import"], &b.ok(&["export"]));
    assert_eq!(a.ok(&["export"]), b.ok(&["export"]));
    assert_eq!(a.ok(&["export"]).lines().count(), 5);
    // Both now show the same fork, with the same winner.
    let (sa, sb) = (a.json(&["of", thing]), b.json(&["of", thing]));
    assert_eq!(sa, sb);
    // Three heads: the two revisions fork, and b's note was never replaced.
    // The note that only mentions the thing is not one of them.
    assert_eq!(len(&sa, &["others"]), 2);
    assert_eq!(at(&sa, &["records"]), &Json::Int(4_i64.into()));
    assert!(!a.ok(&["export", "--of", thing]).contains("said in passing"));
    assert!(
        a.ok(&["export", "--about", thing])
            .contains("said in passing")
    );
    let mut heads = vec![text(&sa, &["winner", "id"])];
    if let Json::Arr(others) = at(&sa, &["others"]) {
        heads.extend(others.iter().map(|line| text(line, &["id"])));
    }
    assert!(heads.contains(&from_a.as_str()) && heads.contains(&from_b.as_str()));
    // Importing again adds nothing, and a line that doesn't check adds nothing at all.
    let before = a.segments().len();
    a.ok_with(&["import"], &b.ok(&["export"]));
    assert_eq!(
        a.segments().len(),
        before,
        "the same lines are the same segment"
    );
    assert_eq!(a.ok(&["export"]), b.ok(&["export"]));
    let forged = b.ok(&["export"]).replace("only b so far", "only b, so far");
    assert_eq!(a.fails(&["import"], &forged).0, 1);
    assert_eq!(a.ok(&["export"]), b.ok(&["export"]));
}

#[test]
fn usage_refusals_and_a_missing_store_exit_apart() {
    let s = Store::new("exits");
    assert_eq!(s.fails(&["frobnicate"], "").0, 2);
    assert_eq!(s.fails(&["emit"], "").0, 2);
    assert_eq!(
        s.fails(&["emit", "note", "--about", "untyped"], ""),
        (1, "refused: about-not-typed".into())
    );
    assert_eq!(
        s.fails(&["emit", "note", "--of", "Not:typed"], ""),
        (1, "refused: of-not-typed".into())
    );
    assert_eq!(
        s.fails(&["get", "zzzz"], ""),
        (1, "refused: id-unknown: zzzz".into())
    );
    assert_eq!(s.fails(&["read", "nothing"], "").0, 1);
    assert_eq!(
        s.fails(&["fold", "-"], r#"{"vertex":"v"}"#),
        (1, "refused: from-missing at from".into())
    );
    // A key writes only as itself until something binds it to an observer; nothing is written.
    let held = s.ok(&["export"]);
    assert_eq!(
        s.fails(&["emit", "note", "--observer", "sl:kyle", "x"], ""),
        (
            1,
            "refused: observer-unbound: sl:kyle is not this key".into()
        )
    );
    assert_eq!(
        s.fails(
            &["emit", "-"],
            "{\"kind\":\"note\",\"observer\":\"sl:kyle\"}\n"
        )
        .0,
        1
    );
    assert_eq!(s.ok(&["export"]), held);
    let nowhere = Store {
        dir: Path::new("/").to_owned(),
    };
    assert_eq!(nowhere.fails(&["export"], "").0, 2);
}
