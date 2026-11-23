//! The TK13 writer reads one local snapshot and records the slice it folded.
use super::{
    Json, SALT, Store, WHEN, assert_cut_report, at, emit_backups, fixed_emit, fixed_store,
    fixed_write, has, parse_json, prev, strings, text,
};

const BACKUPS: &str = r#"{"vertex":"backups","from":[{"store":".","kind":["backup"]}],"fold":{"runs":{"count":true},"mb":{"sum":"mb"}}}"#;
const CURRENT: &str = r#"{"vertex":"current","from":[{"store":".","kind":["backup"]}],"fold":{"current":{"count":true,"follow":"prev"},"mb":{"sum":"mb","follow":"prev"}}}"#;

fn write_tick(s: &Store, spec: &str, flags: &[&str]) -> (String, Json) {
    let args = [&["tick", "-"][..], flags].concat();
    let id = s.ok_with(&args, spec);
    assert_eq!(
        text(&s.json(&["tick", "--check", &id]), &["answer"]),
        "same"
    );
    let line = s.json(&["get", &id]);
    (id, at(&line, &["payload"]).clone())
}

#[test]
fn six_backups_and_the_retired_successor_match_the_local_fold() {
    let s = Store::new("slice-backups");
    let mut backups = emit_backups(&s);
    let first = backups[0].clone();
    s.changeset(&"{\"kind\":\"note\",\"payload\":{\"yard\":\"west\"}}\n".repeat(40));
    let all_store = Store::new("slice-backups-all");
    all_store.ok_with(&["import"], &s.ok(&["export"]));
    let (_, all) = write_tick(&all_store, BACKUPS, &["--all"]);
    let (_, daily) = write_tick(&s, BACKUPS, &[]);
    backups.sort_unstable();
    assert_eq!(strings(&daily, &["covers"]), backups);
    assert_eq!(strings(&all, &["covers"]).len(), 46);
    assert_eq!(text(&daily, &["result-hash"]), text(&all, &["result-hash"]));
    assert_eq!(text(&daily, &["result", "runs", "value"]), "6");
    assert_eq!(text(&daily, &["result", "mb", "value"]), "1027");

    let retired = s.ok(&["emit", "retired", "--prev", &first, "this backup is void"]);
    backups.push(retired);
    backups.sort_unstable();
    let (_, current) = write_tick(&s, CURRENT, &[]);
    assert_eq!(strings(&current, &["covers"]), backups);
    let fold = parse_json(&s.ok_with(&["fold", "-"], CURRENT)).expect("local fold");
    for (slot, expected) in [("current", 5_i64), ("mb", 615)] {
        assert_eq!(
            text(&current, &["result", slot, "value"]),
            expected.to_string()
        );
        assert_eq!(at(&fold, &[slot, "value"]), &Json::Int(expected.into()));
    }
}

#[test]
fn all_matches_the_signed_line_from_main() {
    let s = fixed_store("slice-all-golden");
    fixed_emit(&s, &["backup"]);
    fixed_emit(&s, &["note"]);
    let (id, _) = write_tick(&s, BACKUPS, &["--all", "--emitted", WHEN, "--salt", SALT]);
    let line = s.run(&["get", &id], "");
    assert!(line.status.success());
    // SHA-256 (base32hex) of the entire signed output, including its LF.
    // Captured with the bare command at 07c21f2 on this same fixed holding.
    assert_eq!(
        loops::base32::base32(&loops_place::line::hash(&line.stdout)),
        "6rapmcuieun98t2pq91sm1088rv7he66j337flu5umbj6jq8rn00"
    );
}

#[test]
fn ticks_are_source_records_and_their_covers_are_payload() {
    let s = fixed_store("slice-ticks");
    fixed_emit(&s, &["note"]);
    let (empty_id, empty) = write_tick(&s, BACKUPS, &[]);
    assert_eq!(strings(&empty, &["covers"]), Vec::<String>::new());
    let backup = fixed_emit(&s, &["backup"]);
    let (first, daily) = write_tick(&s, BACKUPS, &[]);
    let fixed = ["--emitted", WHEN, "--salt", SALT];
    let (second, later) = write_tick(&s, BACKUPS, &fixed);
    assert_ne!(first, second);
    assert_eq!(daily, later);
    assert_eq!(strings(&later, &["covers"]), [backup]);
    let (explicit_id, _) = write_tick(&s, BACKUPS, &[&fixed[..], &["--slice"]].concat());
    assert_eq!(second, explicit_id);
    let mut ids = [empty_id, first, second];
    ids.sort_unstable();
    let (_, weekly) = write_tick(
        &s,
        r#"{"vertex":"weekly","from":[{"store":".","kind":["tick"]}],"fold":{"days":{"count":true}}}"#,
        &[],
    );
    assert_eq!(strings(&weekly, &["covers"]), ids);
}

#[test]
fn keyed_slice_covers_the_touched_kilns_history() {
    let s = Store::new("slice-kilns");
    let mut kiln = Vec::new();
    for (kind, field) in [
        ("setup", "heat:=810"),
        ("setup", "heat:=830"),
        ("service", "fuel=wood"),
        ("firing", "batch=clay"),
    ] {
        kiln.push(s.ok(&["emit", kind, "--of", "kiln:east", field]));
    }
    s.ok(&["emit", "setup", "--of", "kiln:west", "heat:=900"]);
    let spec = r#"{"vertex":"heat","from":[{"store":".","kind":["firing"]}],"fold":{"degrees":{"sum":"heat","follow":"keys"}}}"#;
    let (_, payload) = write_tick(&s, spec, &[]);
    kiln.sort_unstable();
    assert_eq!(strings(&payload, &["covers"]), kiln);
    assert_eq!(text(&payload, &["result", "degrees", "value"]), "830");
}

#[test]
fn time_slice_stays_inside_the_prev_only_cut() {
    let s = Store::new("slice-bound");
    let grant = s.ok(&["emit", "grant", "--emitted", "2000-01-03T00:00:00Z"]);
    let backup = s.ok(&[
        "emit",
        "backup",
        "--under",
        &grant,
        "--emitted",
        "2000-01-01T00:00:00Z",
        "mb:=5",
    ]);
    // A second root of the same thing is inside the time cut. Only the
    // outside successor names it, so slicing before cutting would leak it in.
    let peer = s.changeset(&format!(
        r#"{{"kind":"note","of":"{backup}","emitted":"2000-01-01T00:00:00Z"}}"#
    ));
    s.ok(&[
        "emit",
        "retired",
        "--prev",
        &backup,
        "--prev",
        &peer[0],
        "--emitted",
        "2000-01-03T00:00:00Z",
    ]);
    for flags in [
        ["--slice", "--as-of", "2000-01-02"],
        ["--as-of", "2000-01-02", "--slice"],
    ] {
        let (tick, payload) = write_tick(&s, CURRENT, &flags);
        assert_eq!(
            strings(&payload, &["covers"]),
            std::slice::from_ref(&backup)
        );
        assert_eq!(text(&payload, &["result", "current", "value"]), "1");
        assert_cut_report(&s, &tick, "no", 0);
    }
}

#[test]
fn downward_openings_come_from_the_whole_writer_snapshot() {
    let s = Store::new("slice-outside-eraser");
    let root = s.ok(&["emit", "grant"]);
    let grant = s.ok(&["emit", "grant", "--prev", &root]);
    s.ok(&["emit", "withdrawal", "--erases", &root]);
    let backup = s.ok(&["emit", "backup", "--under", &grant]);
    let (id, payload) = write_tick(&s, BACKUPS, &[]);
    let mut expected = [root.clone(), grant, backup];
    expected.sort_unstable();
    assert_eq!(strings(&payload, &["covers"]), expected);
    assert_eq!(strings(&payload, &["withheld"]), [root]);
    assert_cut_report(&s, &id, "yes", 0);
}

#[test]
fn small_tick_base_revises_only_a_thing_it_showed() {
    let s = fixed_store("slice-base");
    fixed_emit(&s, &["plant", "--of", "plant:fern", "water=weekly"]);
    let original = fixed_emit(&s, &["backup", "--of", "backup:kiln", "mb:=3"]);
    let (tick, _) = write_tick(&s, BACKUPS, &[]);
    let other = fixed_write(
        &s,
        &[
            "plant",
            "--of",
            "plant:fern",
            "water=daily",
            "--base",
            &tick,
        ],
        "the base covers no record of plant:fern",
    );
    assert!(!has(&s.json(&["get", &other]), &["env"], "prev"));
    let revision = fixed_write(
        &s,
        &["backup", "--of", "backup:kiln", "mb:=7", "--base", &tick],
        "",
    );
    assert_eq!(prev(&s, &revision), [original]);
}
