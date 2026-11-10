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
  "current":{"latest-by":"about","follow":"prev"}}}"#;

#[test]
fn emit_revise_fork_fold_and_erase() {
    let s = Store::new("decision");
    let thing = "repo:strange-loops";
    let first = s.ok(&[
        "emit",
        "decision",
        "--about",
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
    let second = s.ok(&[
        "emit",
        "decision",
        "--about",
        thing,
        "--prev",
        &first[2..10],
        "status=settled",
        "n:=2",
        "Rust, in a new repo",
    ]);
    let state = s.json(&["about", thing]);
    assert_eq!(text(&state, &["winner", "id"]), second);
    assert_eq!(len(&state, &["others"]), 0);
    let fork = s.ok(&[
        "emit",
        "decision",
        "--about",
        thing,
        "--prev",
        &first,
        "status=settled",
        "n:=4",
        "Zig",
    ]);
    let state = s.json(&["about", thing]);
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
    assert_eq!(text(&s.json(&["about", thing]), &["winner", "id"]), fork);
    let check = s.json(&["check"]);
    assert_eq!(
        (len(&check, &["refused"]), len(&check, &["altered"])),
        (0, 0)
    );
}

#[test]
fn a_batch_is_one_changeset_and_labels_name_its_records() {
    let s = Store::new("batch");
    let drafts = r#"{"label":"@d","kind":"decision","about":["repo:strange-loops"],"payload":{"text":"Rust, in a new repo"}}
{"kind":"note","about":["@d"],"payload":{"text":"phones and extensions need the core itself"}}
{"label":"@d2","kind":"decision","about":["repo:strange-loops"],"prev":["@d"],"payload":{"text":"until the rename"}}
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
    let state = s.json(&["about", "repo:strange-loops"]);
    assert_eq!(text(&state, &["winner", "id"]), text(&records[2], &["id"]));
    // The note is about the first decision's own thing, which the decision anchors.
    assert_eq!(
        at(
            &s.json(&["about", text(&records[0], &["id"])]),
            &["records"]
        ),
        &Json::Int(1_i64.into())
    );

    // A revision written without --about is about what it replaces, here the thing a
    // record about nothing anchors.
    let anchor = s.ok(&["emit", "note", "first"]);
    let revised = s.ok(&["emit", "note", "--prev", &anchor, "second"]);
    let state = s.json(&["about", &anchor]);
    assert_eq!(text(&state, &["winner", "id"]), revised);
    assert_eq!(len(&state, &["others"]), 0);

    // Nothing is written unless every draft is taken.
    let held = s.ok(&["export"]);
    for (bad, code) in [
        (r#"{"kind":"note","about":["@nobody"]}"#, "label-unknown"),
        (r#"{"kind":"","payload":{}}"#, "kind-empty"),
        (
            r#"{"kind":"note","prev":["r:0000000000000000000000000000000000000000000000000000"]}"#,
            "prev-unknown",
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
fn two_stores_converge_by_export_and_import() {
    let (a, b) = (Store::new("sync-a"), Store::new("sync-b"));
    let thing = "repo:strange-loops";
    let root = a.ok(&["emit", "decision", "--about", thing, "start"]);
    b.ok_with(&["import"], &a.ok(&["export"]));
    // Each revises what both hold, apart; a third record exists only in b.
    let from_a = a.ok(&[
        "emit",
        "decision",
        "--about",
        thing,
        "--prev",
        &root,
        "a's revision",
    ]);
    let from_b = b.ok(&[
        "emit",
        "decision",
        "--about",
        thing,
        "--prev",
        &root,
        "b's revision",
    ]);
    b.ok(&["emit", "note", "--about", thing, "only b so far"]);
    assert_ne!(a.ok(&["export"]), b.ok(&["export"]));
    // b can't revise a record it doesn't hold.
    assert_eq!(
        b.fails(&["emit", "note", "--prev", &from_a, "x"], ""),
        (1, "refused: prev-unknown".into())
    );
    b.ok_with(&["import"], &a.ok(&["export"]));
    a.ok_with(&["import"], &b.ok(&["export"]));
    assert_eq!(a.ok(&["export"]), b.ok(&["export"]));
    assert_eq!(a.ok(&["export"]).lines().count(), 4);
    // Both now show the same fork, with the same winner.
    let (sa, sb) = (a.json(&["about", thing]), b.json(&["about", thing]));
    assert_eq!(sa, sb);
    // Three heads: the two revisions fork, and b's note was never replaced.
    assert_eq!(len(&sa, &["others"]), 2);
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
