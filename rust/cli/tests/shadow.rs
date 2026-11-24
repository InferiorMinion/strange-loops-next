//! Check the mapping against fixed signed lines and the changeset filler.
#[path = "../../place/tests/common/mod.rs"]
mod common;
#[path = "../src/shadow/mapping.rs"]
mod mapping;
#[path = "../src/shadow/refs.rs"]
mod refs;
#[path = "../src/shadow/source.rs"]
mod source;

use common::{Rng, emitted};
use loops::{base32, batch, json::Json};
use loops_place::{
    dir::Dir,
    key::Key,
    line::{Line, hash},
};
use mapping::Unnamed::{Omit, Roots};
use mapping::{Fate, History, Prepared, Refusal, Unnamed, occurred};
use source::{Fact, Map, Summary, scan};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

const MAP: &str = include_str!("fixtures/shadow/workshop.map.json");
const FACTS: &str = include_str!("fixtures/shadow/workshop.facts.jsonl");
const LINES: &str = include_str!("fixtures/shadow/expected-lines.jsonl");

fn signer() -> Key {
    Key::from_text(&"0".repeat(52)).expect("fixed changeset signer")
}

fn fact(kind: &str, payload: &str) -> Fact {
    Fact {
        id: "0".repeat(26),
        coordinate: vec![1],
        span: 0..0,
        raw_digest: vec![],
        kind: kind.into(),
        ts_bits: 1.5_f64.to_bits(),
        observer: "yard:west".into(),
        origin: String::new(),
        signature: None,
        payload: payload.into(),
    }
}

// Exercise the same classification and preparation as the group import with
// this test's single in-memory fact in place of a verified export slice.
#[derive(Debug)]
enum Row {
    Included(Box<Prepared>),
    Omitted { defect: Option<Refusal> },
}

impl History {
    fn prepare(
        &mut self,
        map: &Map,
        unnamed: Unnamed,
        fact: &Fact,
        key: &Key,
        salt: &[u8],
        emitted: &str,
    ) -> Result<Row, Refusal> {
        let payload = fact.parsed_payload();
        match mapping::classify(map, unnamed, fact, payload.as_ref()) {
            Fate::Omitted { defect } => Ok(Row::Omitted { defect }),
            Fate::Included(row) => {
                let row = row?;
                let own = match &row.target {
                    mapping::Target::Keyed(t) => Some(t.as_str()),
                    _ => None,
                };
                let payload = payload.expect("classified payload");
                let changes =
                    refs::Changes::resolve(&refs::Field::read(&payload, map), own, |id| match id {
                        "same-root" => refs::Lookup::SelfReference,
                        "potter-id" => refs::Lookup::Target("person:potter".into()),
                        _ => refs::Lookup::Nonmember,
                    });
                self.prepare_included(&row, payload, changes, key, salt, emitted)
                    .map(|p| Row::Included(Box::new(p)))
            }
        }
    }
}

fn included(row: Row) -> Prepared {
    let Row::Included(prepared) = row else {
        panic!("expected an included row")
    };
    *prepared
}

fn sign(prepared: &Prepared) -> Line {
    let d = &prepared.draft;
    let w =
        loops::stored::write(d.env.clone(), d.salt.clone(), d.payload.clone()).expect("writable");
    let line = Line::sign(&w, &signer());
    assert_eq!(line.id(), prepared.id);
    line
}

fn check_recipe<const N: usize>(original: &str, edits: [(&str, Option<Json>); N], expected: &str) {
    let recipe = refs::PayloadRecipe {
        edits: edits
            .into_iter()
            .map(|(name, edit)| (name.to_owned(), edit))
            .collect(),
    };
    let original = loops_eval::parse_writer_payload(original).expect("original payload");
    let edited = recipe.apply(original.clone());
    // Structural equality checks member order, which canonical JSON sorts away.
    assert_eq!(
        edited,
        loops_eval::parse_writer_payload(expected).expect("expected payload")
    );
    assert_eq!(recipe.apply(original).canon(), edited.canon());
}

#[test]
fn recipe_removes_only_the_named_member() {
    check_recipe(
        r#"{"yard":{"ref":"potter"},"ref":"potter","kiln":true}"#,
        [("ref", None)],
        r#"{"yard":{"ref":"potter"},"kiln":true}"#,
    );
}

#[test]
fn recipe_replaces_text_with_text_or_a_list_in_place() {
    check_recipe(
        r#"{"yard":"east","owner":"potter","backups":false}"#,
        [("owner", Some(Json::Str("person:potter".into())))],
        r#"{"yard":"east","owner":"person:potter","backups":false}"#,
    );
    check_recipe(
        r#"{"yard":"east","kilns":"kiln/b,kiln/a","backups":false}"#,
        [(
            "kilns",
            Some(Json::Arr(vec![
                Json::Str("kiln:b".into()),
                Json::Str("kiln:a".into()),
            ])),
        )],
        r#"{"yard":"east","kilns":["kiln:b","kiln:a"],"backups":false}"#,
    );
}

#[test]
fn recipe_applies_removal_and_replacement_together() {
    check_recipe(
        r#"{"yard":"east","owner_ref":"potter","owner":"person/potter","backups":[]}"#,
        [
            ("owner", Some(Json::Str("person:potter".into()))),
            ("owner_ref", None),
        ],
        r#"{"yard":"east","owner":"person:potter","backups":[]}"#,
    );
}

#[test]
fn exporter_cut_produces_known_signed_lines() {
    let map = Map::read(MAP).expect("reviewed map");
    assert_ne!(map.semantic_pin, "");
    assert_eq!(map.store, "workshop.sqlite");
    let mut history = History::default();
    let mut lines = Vec::new();
    let mut coords = Vec::new();
    let summary = scan(FACTS.as_bytes(), &map, |fact| {
        assert_eq!(fact.origin, "");
        assert_eq!(fact.signature, None);
        assert_eq!(fact.observer, "");
        let i = lines.len();
        let row = history
            .prepare(
                &map,
                Omit,
                &fact,
                &signer(),
                &[u8::try_from(i).expect("small"); 16],
                &emitted(i),
            )
            .expect("accepted");
        let prepared = included(row);
        assert!(prepared.stats.is_empty() && prepared.recipe.removed().next().is_none());
        assert!(!prepared.missing_key);
        lines.push(sign(&prepared).render());
        coords.push(fact.coordinate);
    })
    .expect("valid cut");
    assert_eq!(summary, Summary { facts: 3, ticks: 1 });
    assert_eq!(coords, [vec![5, 0], vec![5, 1], vec![9, 2]]);
    assert_eq!(lines, LINES.lines().collect::<Vec<_>>());
}

#[test]
fn framing_binding_and_coordinates_are_checked() {
    let map = Map::read(MAP).expect("map");
    let first = FACTS.lines().nth(1).expect("row");
    let second = FACTS.lines().nth(2).expect("row");
    let footer = FACTS.lines().last().expect("footer");
    for broken in [
        String::new(),
        FACTS.trim_end().into(),
        FACTS.replace(footer, ""),
        format!("{FACTS}\n"),
        FACTS.replace("shadow-facts-v1", "v2"),
        FACTS.replace("workshop", "yard"),
        FACTS.replace(&map.vertex_sha256, &"a".repeat(64)),
        FACTS.replace("arrival", "unknown"),
        FACTS.replace("\"ticks\":\"1\"", "\"ticks\":null"),
        FACTS.replace("\"count\":\"3\"", "\"count\":\"2\""),
        FACTS.replace(first, second),
        FACTS.replace("[\"5\",\"1\"]", "[\"4\",\"0\"]"),
        FACTS.replace("[\"5\",\"1\"]", "[\"5\",\"0\"]"),
        FACTS.replace("[\"5\",\"0\"]", "[null,\"0\"]"),
        FACTS.replace("[\"5\",\"0\"]", "[\"5\"]"),
        FACTS.replace("arrival", "rowid"),
        FACTS.replace("00000000000000000000000030", "00000000000000000000000011"),
        FACTS.replace("3ff8000000000000", "3fg8000000000000"),
        FACTS.replace("\"origin\":\"\"", "\"origin\":\"yard\""),
        FACTS.replace(
            "\"signature\":null",
            "\"signature\":null,\"signature\":null",
        ),
    ] {
        match scan(broken.as_bytes(), &map, |_| {}).expect_err("bad cut") {
            source::ScanError::Invalid(why) => assert!(why.starts_with("shadow-source-invalid")),
            source::ScanError::Io(e) => panic!("unexpected I/O error: {e}"),
        }
    }
    for broken in [
        MAP.replace("shadow-map-v1", "v2"),
        MAP.replace("\"thread\":", "\"change\":"),
        MAP.replace("\"collect\":\"2\"", "\"sum\":\"2\""),
        MAP.replace("\"by\":\"name\"", "\"by\":\"\""),
        MAP.replace("\"workshop\"", "\"../yard\""),
        MAP.replace("\"collect\":\"2\"", "\"collect\":\"02\""),
        MAP.replace("\"kinds\":", "\"kinds\":{},\"kinds\":"),
    ] {
        assert!(
            Map::read(&broken)
                .expect_err("bad map")
                .starts_with("shadow-map-invalid")
        );
    }
}

#[test]
fn microseconds_round_the_original_binary64_once() {
    for (value, instant) in [
        (0., "1970-01-01T00:00:00Z"),
        (-0.25, "1969-12-31T23:59:59.75Z"),
        (1.0 / 128., "1970-01-01T00:00:00.007812Z"),
        (3.0 / 128., "1970-01-01T00:00:00.023438Z"),
        (-3.0 / 128., "1969-12-31T23:59:59.976562Z"),
        (0.999_999_9, "1970-01-01T00:00:01Z"),
        (-62_167_219_200., "0000-01-01T00:00:00Z"),
        (253_402_300_799., "9999-12-31T23:59:59Z"),
    ] {
        assert_eq!(occurred(f64::to_bits(value)).as_deref(), Some(instant));
    }
    for invalid in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -62_167_219_201.,
        253_402_300_800.,
    ] {
        assert_eq!(occurred(invalid.to_bits()), None);
    }
}

#[test]
fn roots_and_omissions() {
    let map = Map::read(MAP).expect("map");
    let key = signer();
    let mut h = History::default();
    for (kind, payload, missing) in [
        ("thread", r#"{"message":"kiln"}"#, true),
        ("change", r#"{"name":"kiln"}"#, false),
        ("moved", r#"{"name":"kiln"}"#, false),
    ] {
        let p = included(
            h.prepare(
                &map,
                Roots,
                &fact(kind, payload),
                &key,
                &[1; 16],
                &emitted(1),
            )
            .expect("root"),
        );
        assert_eq!(p.missing_key, missing);
        assert_eq!(p.draft.env.of, None);
        assert_eq!(p.draft.env.prev, Vec::<String>::new());
        assert_eq!(p.draft.env.sets, None);
        assert_eq!(p.draft.env.about, ["observer:yard:west"]);
        assert_eq!(p.draft.env.observer, key.public());
        assert_eq!(
            p.draft.env.occurred.as_deref(),
            Some("1970-01-01T00:00:01.5Z")
        );
    }
    for (payload, defect) in [
        ("{}", None),
        ("[]", Some("shadow-source-invalid")),
        (r#"{"nest":[{"x":1,"x":2}]}"#, Some("member-duplicate")),
        (r#"{"n":1e400}"#, Some("number-out-of-range")),
    ] {
        let Row::Omitted { defect: actual } = h
            .prepare(&map, Omit, &fact("moved", payload), &key, &[], "")
            .expect("omitted")
        else {
            panic!("unnamed kind should be omitted")
        };
        assert_eq!(actual.as_ref().map(mapping::Refusal::code), defect);
    }
}

#[test]
fn unrepresentable_facts_leave_the_history_unchanged() {
    let mut map = Map::read(MAP).expect("map");
    let key = signer();
    let mut h = History::default();
    for (kind, payload, code) in [
        ("thread", r#"{"name":null}"#, "shadow-source-invalid"),
        ("thread", r#"{"name":2}"#, "shadow-source-invalid"),
        ("thread", r#"{"name":""}"#, "of-not-typed"),
        ("tick", "{}", "shadow-source-invalid"),
        ("", "{}", "kind-empty"),
        ("thread", "[]", "shadow-source-invalid"),
        ("thread", "{", "shadow-source-invalid"),
        ("thread", r#"{"x":1,"x":2}"#, "member-duplicate"),
        (
            "thread",
            r#"{"x":[9007199254740992]}"#,
            "number-out-of-range",
        ),
        ("thread", r#"{"x":[1e400]}"#, "number-out-of-range"),
    ] {
        assert_eq!(
            h.prepare(
                &map,
                Roots,
                &fact(kind, payload),
                &key,
                &[0; 16],
                &emitted(1)
            )
            .expect_err("refused")
            .code(),
            code
        );
    }
    for (kind, code) in [
        ("r", "shadow-map-invalid"),
        ("thread:hot", "shadow-map-invalid"),
        ("Kiln", "of-not-typed"),
    ] {
        map.kinds
            .insert(kind.into(), source::Fold::By("kiln".into()));
        assert_eq!(
            h.prepare(&map, Roots, &fact(kind, "{}"), &key, &[0; 16], &emitted(1))
                .expect_err("invalid keyed kind")
                .code(),
            code
        );
    }
    let first = fact("thread", r#"{"name":"kiln","":0}"#);
    let base = included(
        h.prepare(&map, Omit, &first, &key, &[0; 16], &emitted(0))
            .expect("whole may hold empty key"),
    );
    assert_eq!(
        h.prepare(&map, Omit, &first, &key, &[1; 16], &emitted(1))
            .expect_err("sets cannot")
            .code(),
        "sets-not-keys"
    );
    let next = included(
        h.prepare(
            &map,
            Omit,
            &fact("thread", r#"{"name":"kiln"}"#),
            &key,
            &[2; 16],
            &emitted(2),
        )
        .expect("retry"),
    );
    assert_eq!(next.draft.env.prev, [base.id]);
}

#[test]
fn group_reservations_work_with_local_histories() {
    let map = Map::read(MAP).expect("map");
    let mut workshop = History::default();
    let mut yard = History::default();
    let prepare = |history: &mut History, old_id: u8| {
        let mut row = fact("thread", r#"{"name":"kiln","status":"open"}"#);
        row.id = format!("{old_id:026}");
        included(
            history
                .prepare(
                    &map,
                    Omit,
                    &row,
                    &signer(),
                    &[old_id; 16],
                    &emitted(usize::from(old_id)),
                )
                .expect("row"),
        )
    };
    assert_eq!(
        sign(&prepare(&mut workshop, 1)),
        sign(&prepare(&mut yard, 1))
    );
    let workshop_only = prepare(&mut workshop, 2);
    let yard_only = prepare(&mut yard, 3);
    assert_eq!(workshop_only.draft.env.of, yard_only.draft.env.of);
    assert_ne!(workshop_only.id, yard_only.id);
    let workshop_copy = prepare(&mut workshop, 4);
    let yard_copy = prepare(&mut yard, 4);
    assert_eq!(workshop_copy.draft.salt, yard_copy.draft.salt);
    assert_eq!(workshop_copy.draft.env.emitted, yard_copy.draft.env.emitted);
    assert_ne!(workshop_copy.draft.env.prev, yard_copy.draft.env.prev);
    assert_ne!(workshop_copy.id, yard_copy.id);
}

fn emit_reference(dir: &Path, input: &Json) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_loops"))
        .arg("-C")
        .arg(dir)
        .args(["emit", "-", "--fill"])
        .env("HOME", dir.join("home"))
        .env("XDG_CONFIG_HOME", dir.join("home/xdg-config"))
        .env_remove("XDG_CACHE_HOME")
        .env("LOOPS_CACHE_DIR", dir.join("cache"))
        .env_remove("LOOPS_KEY")
        .env_remove("LOOPS_OBSERVER")
        .env("LOOPS_KEY_FILE", dir.join("device-key"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run loops");
    writeln!(child.stdin.take().expect("stdin"), "{}", input.canon())
        .expect("supply reference statement");
    let output = child.wait_with_output().expect("wait for CLI");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json = loops_eval::parse_json(std::str::from_utf8(&output.stdout).expect("UTF-8"))
        .expect("response");
    let Json::Obj(fields) = json else {
        panic!("expected object")
    };
    let Json::Arr(rows) = &fields[0].1 else {
        panic!("expected records")
    };
    let Json::Obj(record) = &rows[0] else {
        panic!("expected row")
    };
    let Json::Str(id) = &record[0].1 else {
        panic!("expected id")
    };
    id.clone()
}

fn reference_inputs(step: usize, payload: &mut Vec<(String, Json)>) -> (Json, Option<Json>) {
    let mut original = payload.clone();
    if step == 0 {
        let kept = ("ref".into(), Json::Str("?".into()));
        original.push(kept.clone());
        payload.push(kept);
    } else if step == 3 {
        original.push(("ref".into(), Json::Str("thread/yard".into())));
    }
    if step == 5 {
        original.push(("ref".into(), Json::Str("same-root".into())));
    }
    if step == 1 || step == 4 {
        let owner = ("owner".into(), Json::Str("person/potter".into()));
        original.push(owner.clone());
        payload.push(owner);
        original.push((
            "owner_ref".into(),
            Json::Str(if step == 1 { "potter-id" } else { "" }.into()),
        ));
    }
    let clearing = (step == 3).then(|| {
        let mut keys: Vec<_> = payload.iter().map(|(k, _)| Json::Str(k.clone())).collect();
        keys.push(Json::Str("ref".into()));
        Json::Arr(keys)
    });
    (Json::Obj(original), clearing)
}

#[test]
fn prepared_lines_equal_successive_cli_fills() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("shadow-differential");
    let _ = fs::remove_dir_all(&root);
    let dir = Dir::init(&root.join(".loops")).expect("empty reference store");
    fs::write(root.join("device-key"), "0".repeat(52)).expect("oracle signer");
    let map = Map::read(MAP).expect("map");
    let mut history = History::default();
    let key = signer();
    let mut rng = Rng(0x2026_1120_d8c0_0005);
    let mut prepared = Vec::new();
    for step in 0..48 {
        let name = ["kiln", "workshop", "yard"][step % 3];
        let kind = if step % 11 == 5 { "change" } else { "thread" };
        let mut payload: Vec<(String, Json)> = vec![(
            rng.pick(&["message", "heat", "status"]).into(),
            Json::Str(step.to_string()),
        )];
        let has_key = step % 13 != 6;
        if has_key {
            payload.push(("name".into(), Json::Str(name.into())));
        }
        let (original, clearing) = reference_inputs(step, &mut payload);
        let payload = Json::Obj(payload);
        let mut fact = fact(kind, &original.canon());
        if rng.below(3) == 0 || step == 4 {
            fact.observer.clear();
        }
        let mut about = if fact.observer.is_empty() {
            Vec::new()
        } else {
            vec![Json::Str("observer:yard:west".into())]
        };
        if step >= 3 && name == "kiln" && kind == "thread" && has_key {
            about.push(Json::Str("thread:yard".into()));
        }
        if step == 1 {
            about.push(Json::Str("person:potter".into()));
        }
        let time = if step % 2 == 0 {
            fact.ts_bits = (-0.25_f64).to_bits();
            "1969-12-31T23:59:59.75Z"
        } else {
            "1970-01-01T00:00:01.5Z"
        };
        let salt = [rng.next().to_be_bytes(), rng.next().to_be_bytes()].concat();
        let mut reference = vec![
            ("kind".into(), Json::Str(kind.into())),
            ("payload".into(), payload),
            ("salt".into(), Json::Str(base32::base32(&salt))),
            ("about".into(), Json::Arr(about)),
            ("emitted".into(), Json::Str(emitted(step))),
            ("occurred".into(), Json::Str(time.into())),
        ];
        if let Some(sets) = clearing {
            reference.push(("sets".into(), sets));
        }
        if kind == "thread" && has_key {
            reference.push(("of".into(), Json::Str(format!("thread:{name}"))));
        }
        let expected_id = emit_reference(&root, &Json::Obj(reference));
        let row = included(
            history
                .prepare(&map, Omit, &fact, &key, &salt, &emitted(step))
                .expect("accepted"),
        );
        assert_eq!(row.id, expected_id, "step {step}");
        prepared.push(row.draft);
    }
    // Exercise the real checked batch over literals, including cross-batch lookup.
    let reference = dir.read().expect("read oracle's stored bytes");
    let mut committed = HashMap::new();
    for drafts in prepared.chunks(7) {
        let lines = batch::resolve_batch(&hash, &|id| committed.get(id).cloned(), drafts)
            .expect("checked batch");
        for w in lines {
            let actual = Line::sign(&w, &key);
            assert_eq!(
                actual.render(),
                reference
                    .line(actual.id())
                    .expect("oracle holds id")
                    .render()
            );
            committed.insert(
                actual.id().to_owned(),
                batch::Held {
                    of: actual.env().of.clone(),
                    prev: actual.env().prev.clone(),
                },
            );
        }
    }
    assert_eq!(reference.lines().len(), committed.len());
}

#[test]
fn repeated_sets_retire_an_old_patch_but_the_first_whole_stays_live() {
    let map = Map::read(&MAP.replace("\"by\":\"name\"", "\"by\":\"slot\"")).expect("map");
    let key = signer();
    let mut history = History::default();
    let mut rows = Vec::new();
    for (step, data) in [
        r#"{"slot":"kiln","name":"old","heat":20}"#,
        r#"{"slot":"kiln","status":"open"}"#,
        r#"{"slot":"kiln","name":"new"}"#,
        r#"{"slot":"kiln","status":null}"#,
        r#"{"slot":"kiln","heat":0}"#,
    ]
    .iter()
    .enumerate()
    {
        let p = history
            .prepare(
                &map,
                Omit,
                &fact("thread", data),
                &key,
                &[0; 16],
                &emitted(step),
            )
            .expect("patch");
        rows.push(included(p));
    }
    assert_eq!(rows[0].draft.env.of.as_deref(), Some("thread:kiln"));
    assert_eq!(rows[0].draft.env.sets, None);
    assert_eq!(
        rows[3].draft.env.sets.as_deref(),
        Some(["slot".into(), "status".into()].as_slice())
    );
    for (step, live) in [(3, [0, 1, 2]), (4, [0, 2, 3])] {
        let expected = live.map(|i: usize| rows[i].id.clone());
        assert_eq!(rows[step].draft.env.prev, loops::json::canon_set(&expected));
    }
}

#[test]
fn contemporary_epoch_rounding_avoids_an_extra_binary64_round() {
    // Scaling this binary64 by a million in f64 would round it up to a half.
    assert_eq!(
        occurred(0x41d9_54fc_4000_0006).as_deref(),
        Some("2023-11-14T22:13:20.000001Z")
    );
}

#[test]
fn legacy_cut_retains_rowid_gaps_in_its_arrival_order() {
    let map = Map::read(MAP).expect("map");
    let cut = include_bytes!("fixtures/shadow/workshop.rowid.facts.jsonl");
    let mut rows = Vec::new();
    let summary = scan(cut.as_slice(), &map, |fact| rows.push(fact.coordinate)).expect("rowid cut");
    assert_eq!(rows, [vec![4], vec![11], vec![30]]);
    assert_eq!(summary, Summary { facts: 3, ticks: 1 });
}

#[test]
fn invalid_and_duplicate_old_ids_name_the_fact_line() {
    let map = Map::read(MAP).expect("map");
    let long = "x".repeat(129);
    for (id, line) in [
        ("", 2),
        (long.as_str(), 2),
        ("kiln yard", 2),
        ("kiln\n", 2),
        ("kiln\x7f", 2),
        ("café", 2),
        ("00000000000000000000000011", 3),
    ] {
        let cut = FACTS.replace("\"00000000000000000000000030\"", &loops::json::quote(id));
        let source::ScanError::Invalid(why) =
            scan(cut.as_bytes(), &map, |_| {}).expect_err("bad id")
        else {
            panic!("unexpected I/O error");
        };
        assert!(
            why.starts_with(&format!("shadow-source-invalid at line {line}:")),
            "{why}"
        );
    }
}
