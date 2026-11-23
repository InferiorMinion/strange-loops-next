//! End-to-end import, recovery boundaries and AD1 union reads.
use super::{Store, answer, at, strings, text};
use loops::json::{Json, quote};
use loops_place::line::hash;
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

const MAP: &str = include_str!("../fixtures/shadow/workshop.map.json");
const VERB: &[&str] = &["shadow-import", "group.json"];
type Old<'a> = (usize, &'a str, &'a str, &'a str);
const START: Old<'_> = (1, "thread", "yard", r#"{"name":"kiln","message":"fire"}"#);
const END: Old<'_> = (3, "thread", "", "{\"name\":\"kiln\",\"status\":\"done\"}");
const WORKSHOP: &[Old<'_>] = &[
    START,
    (
        2,
        "thread",
        "workshop",
        r#"{"name":"kiln","temperature":"hot"}"#,
    ),
    END,
    (7, "change", "", "{}"),
    (8, "thread", "yard", "{}"),
    (9, "moved", "yard", r#"{"name":"kiln"}"#),
];
const YARD: &[Old<'_>] = &[
    START,
    (
        4,
        "thread",
        "yard",
        r#"{"name":"kiln","temperature":"cool"}"#,
    ),
    END,
];

fn cut(root: &Path, name: &str, rows: &[Old<'_>]) {
    let map = MAP.replace("workshop", name);
    fs::write(root.join(format!("{name}.map")), &map).expect("map");
    let vertex = loops_eval::parse_json(&map).expect("map json");
    let mut bytes=format!(r#"{{"format":"shadow-facts-v1","order":"rowid","source":"{name}","ticks":"2","vertex_sha256":"{}"}}"#,text(&vertex,&["vertex_sha256"])).into_bytes();
    bytes.push(b'\n');
    let mut digest = hash(&bytes);
    for (pos, (id, kind, observer, payload)) in rows.iter().enumerate() {
        let mut line = format!(
            r#"{{"coordinate":["{}"],"id":"{id:026}","kind":"{kind}","observer":"{observer}","origin":"","payload":{},"signature":null,"ts_bits":"3ff8000000000000"}}"#,
            pos + 1,
            quote(payload)
        );
        line.push('\n');
        bytes.extend_from_slice(line.as_bytes());
        digest.extend_from_slice(line.as_bytes());
        digest = hash(&digest);
    }
    let hex = digest.iter().fold(String::new(), |mut s, b| {
        write!(s, "{b:02x}").expect("string");
        s
    });
    writeln!(bytes, r#"{{"count":"{}","digest":"{hex}"}}"#, rows.len()).expect("trailer");
    fs::write(root.join(format!("{name}.facts")), bytes).expect("cut");
}
fn manifest(s: &Store, names: &[&str]) {
    let sources: Vec<_> = names
        .iter()
        .map(|n| {
            format!(r#"{{"name":"{n}","map":"{n}.map","facts":"{n}.facts","destination":"{n}"}}"#)
        })
        .collect();
    fs::write(
        s.dir.join("group.json"),
        format!(r#"{{"state":"private","sources":[{}]}}"#, sources.join(",")),
    )
    .expect("group");
}
fn destination(s: &Store, name: &str) -> Store {
    Store {
        dir: s.dir.join(name),
        scratch: s.scratch.clone(),
    }
}
fn setup(name: &str) -> Store {
    let s = Store::scratch(name);
    for (n, rows) in [("workshop", WORKSHOP), ("yard", YARD)] {
        fs::create_dir_all(s.dir.join(n)).expect("dir");
        destination(&s, n).ok(&["init"]);
        cut(&s.dir, n, rows);
    }
    manifest(&s, &["workshop", "yard"]);
    s
}
fn snapshot(s: &Store) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut paths = destination(s, "workshop").segments();
    paths.extend(destination(s, "yard").segments());
    let state = s.dir.join("private");
    if state.exists() {
        paths.extend(
            fs::read_dir(state)
                .expect("state")
                .map(|e| e.expect("entry").path()),
        );
    }
    paths
        .into_iter()
        .map(|p| {
            let bytes = fs::read(&p).expect("file");
            (p, bytes)
        })
        .collect()
}
fn total(json: &Json, name: &str) -> usize {
    at(json, &["group", name]).canon().parse().expect("count")
}
fn exported(s: &Store) -> Vec<Json> {
    s.ok(&["export"])
        .lines()
        .map(|l| loops_eval::parse_json(l).expect("record"))
        .collect()
}
fn msg<'a>(lines: &'a [Json], field: &str) -> &'a Json {
    lines
        .iter()
        .find(|j| super::has(j, &["payload"], field))
        .expect("record carrying field")
}

#[test]
fn shadow_group_read_and_idempotence() {
    let s = setup("shadow-group");
    let before = snapshot(&s);
    let dry = s.json(&["shadow-import", "group.json", "--check-only"]);
    assert_eq!(total(&dry, "would_write"), 8);
    assert_eq!(total(&dry, "written"), 0);
    assert_eq!(before, snapshot(&s));
    assert!(!s.dir.join("private").exists());
    let result = s.json(VERB);
    for (k, n) in [
        ("facts_read", 9),
        ("written", 8),
        ("without_key", 1),
        ("empty_observer", 3),
        ("unnamed_left_out", 1),
        ("old_ticks_left_out", 4),
        ("destination_reads", 2),
        ("shared_old_ids", 2),
        ("shared_old_ids_one_record", 1),
        ("shared_old_ids_diverged", 1),
        ("things_in_multiple_stores", 1),
        ("things_with_multiple_union_heads", 1),
        ("shared_keyed_things_different_old_ids", 1),
    ] {
        assert_eq!(total(&result, k), n, "{k}");
    }
    let a = destination(&s, "workshop");
    let b = destination(&s, "yard");
    for (d, n) in [(&a, 5), (&b, 3)] {
        assert_eq!(at(&d.json(&["check"]), &["records"]).canon(), n.to_string());
    }
    let ls = a.json(&["ls", "thread:", "--fields", "status,message,temperature"]);
    let Json::Arr(rows) = at(&ls, &["things"]) else {
        panic!("things")
    };
    for (k, v) in [
        ("status", "done"),
        ("message", "fire"),
        ("temperature", "hot"),
    ] {
        assert_eq!(text(&rows[0], &["fields", k, "value"]), v);
    }
    let left = exported(&a);
    let right = exported(&b);
    assert_eq!(msg(&left, "message"), msg(&right, "message"));
    assert_ne!(
        text(msg(&left, "status"), &["id"]),
        text(msg(&right, "status"), &["id"])
    );
    for path in [&["salt"][..], &["env", "emitted"]] {
        assert_eq!(
            text(msg(&left, "status"), path),
            text(msg(&right, "status"), path)
        );
    }
    let spec = r#"{"vertex":"kiln","from":[{"store":"workshop"},{"store":"yard"}],"fold":{"items":{"latest-by":"of","follow":"keys"}}}"#;
    let union = loops_eval::parse_json(&s.ok_with(
        &[
            "fold",
            "-",
            "--store",
            "workshop=workshop",
            "--store",
            "yard=yard",
        ],
        spec,
    ))
    .expect("fold");
    let Json::Arr(heads) = at(&union, &["items", "things", "thread:kiln", "heads"]) else {
        panic!("heads")
    };
    assert_eq!(heads.len(), 2);
    let published = snapshot(&s);
    assert_eq!(total(&s.json(VERB), "already_imported"), 8);
    assert_eq!(published, snapshot(&s));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let state = s.dir.join("private");
        assert_eq!(
            fs::metadata(&state).unwrap().permissions().mode() & 0o777,
            0o700
        );
        for f in ["lock", "key", "journal.jsonl"] {
            assert_eq!(
                fs::metadata(state.join(f)).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}
#[test]
fn shadow_append_and_drift() {
    let s = setup("shadow-append");
    s.ok(VERB);
    let initial = exported(&destination(&s, "workshop"));
    let mut more = WORKSHOP.to_vec();
    more.extend([
        (21, "thread", "yard", r#"{"name":"kiln","first":"next"}"#),
        (22, "change", "yard", "{}"),
        (23, "thread", "yard", r#"{"name":"kiln","status":"next"}"#),
    ]);
    cut(&s.dir, "workshop", &more);
    assert_eq!(total(&s.json(VERB), "written"), 3);
    let all = exported(&destination(&s, "workshop"));
    assert!(
        strings(msg(&all, "first"), &["env", "prev"])
            .contains(&text(msg(&initial, "status"), &["id"]).to_owned())
    );
    let baseline = snapshot(&s);
    let mut edited = more.clone();
    edited[0].2 = "changed";
    let mut reordered = more.clone();
    reordered.swap(0, 1);
    let mut inserted = more.clone();
    inserted.insert(0, (31, "change", "", "{}"));
    for changed in [edited, more[..more.len() - 1].to_vec(), reordered, inserted] {
        cut(&s.dir, "workshop", &changed);
        assert!(s.fails(VERB, "").1.contains("shadow-state-mismatch"));
        assert_eq!(baseline, snapshot(&s));
    }
    cut(&s.dir, "workshop", &more);
    assert!(
        s.fails(&["shadow-import", "group.json", "--include-unnamed"], "")
            .1
            .contains("shadow-state-mismatch")
    );
    for changed_map in [format!("{MAP}\n"), "{".to_owned()] {
        fs::write(s.dir.join("workshop.map"), changed_map).unwrap();
        assert!(s.fails(VERB, "").1.contains("shadow-state-mismatch"));
    }
    assert_eq!(baseline, snapshot(&s));
    cut(&s.dir, "workshop", &more);
    destination(&s, "yard").ok(&["emit", "change", "unreserved"]);
    let extra = snapshot(&s);
    assert!(s.fails(VERB, "").1.contains("unreserved"));
    assert_eq!(extra, snapshot(&s));
}
#[test]
fn shadow_first_run_refusals_and_roots() {
    let s = setup("shadow-refusal");
    let before = snapshot(&s);
    let mut rows = YARD.to_vec();
    rows.push((15, "thread", "", r#"{"name":null}"#));
    cut(&s.dir, "yard", &rows);
    let (code, out, why) = answer(&s, VERB, "");
    assert_eq!(code, 1);
    assert!(why.contains(&format!("yard [4] {:026}", 15)), "{why}");
    assert_eq!(total(&loops_eval::parse_json(&out).unwrap(), "refused"), 1);
    assert_eq!(snapshot(&s), before);
    assert!(!s.dir.join("private").exists());
    rows[0].2 = "conflict";
    cut(&s.dir, "yard", &rows);
    assert!(s.fails(VERB, "").1.contains("shadow-id-conflict"));
    assert_eq!(snapshot(&s), before);
    cut(&s.dir, "yard", YARD);
    assert_eq!(
        total(
            &s.json(&["shadow-import", "group.json", "--include-unnamed"]),
            "unnamed_included"
        ),
        1
    );
    let d = destination(&s, "workshop");
    let root = d.ok(&["export", "--kind", "moved"]);
    assert!(!super::has(
        &loops_eval::parse_json(&root).unwrap(),
        &["env"],
        "of"
    ));
}
#[test]
fn shadow_extend_group_and_recover_torn_tail() {
    let s = setup("shadow-grow");
    manifest(&s, &["workshop"]);
    s.ok(VERB);
    let old = exported(&destination(&s, "workshop"));
    manifest(&s, &["workshop", "yard"]);
    assert_eq!(total(&s.json(VERB), "written"), 3);
    assert_eq!(
        msg(&old, "message"),
        msg(&exported(&destination(&s, "yard")), "message")
    );
    let published = snapshot(&s);
    let journal = s.dir.join("private/journal.jsonl");
    for frame in fs::read_to_string(&journal).unwrap().lines() {
        assert!(matches!(loops_eval::parse_json(frame), Ok(Json::Arr(pair)) if pair.len() == 2));
    }
    fs::OpenOptions::new()
        .append(true)
        .open(&journal)
        .unwrap()
        .write_all(b"torn")
        .unwrap();
    let torn = snapshot(&s);
    s.ok(&["shadow-import", "group.json", "--check-only"]);
    assert_eq!(torn, snapshot(&s));
    s.ok(VERB);
    assert_eq!(published, snapshot(&s));
    let mut damaged = fs::read(&journal).unwrap();
    damaged[0] ^= 1;
    fs::write(&journal, &damaged).unwrap();
    assert!(s.fails(VERB, "").1.contains("damaged journal"));
    assert_eq!(damaged, fs::read(journal).unwrap());
}

#[test]
fn filtered_identity_conflicts_and_direct_destination_binding() {
    let s = setup("shadow-filtered-conflict");
    manifest(&s, &["workshop"]);
    s.ok(VERB);
    let before = snapshot(&s);
    manifest(&s, &["yard"]);
    cut(&s.dir, "yard", &[(9, "moved", "yard", "{}")]);
    let (code, _, why) = answer(&s, VERB, "");
    assert_eq!(code, 1);
    assert!(why.contains("shadow-id-conflict"));
    assert_eq!(snapshot(&s), before);
    let group = s.dir.join("group.json");
    let valid = fs::read_to_string(&group).unwrap();
    fs::create_dir(s.dir.join("yard/child")).unwrap();
    fs::write(
        &group,
        valid.replace(r#""destination":"yard""#, r#""destination":"yard/child""#),
    )
    .unwrap();
    assert_eq!(answer(&s, VERB, "").0, 1);
    assert_eq!(snapshot(&s), before);
    fs::write(&group, valid.replace("yard.facts", "absent.facts")).unwrap();
    assert_eq!(answer(&s, VERB, "").0, 3);
    assert_eq!(
        answer(&s, &["shadow-import", "group.json", "--unknown"], "").0,
        2
    );
}

#[test]
fn large_facts_split_segments_and_nested_state_stays_private() {
    let s = setup("shadow-budget");
    manifest(&s, &["yard"]);
    let group = s.dir.join("group.json");
    let names = fs::read_to_string(&group)
        .unwrap()
        .replace(r#""private""#, r#""nested/private""#);
    fs::write(&group, names).unwrap();
    let large = format!(
        r#"{{"name":"kiln","message":"{}"}}"#,
        "x".repeat(2 * 1024 * 1024)
    );
    cut(
        &s.dir,
        "yard",
        &[
            (1, "thread", "", "{}"),
            (2, "thread", "", &large),
            (3, "thread", "", &large),
        ],
    );
    s.ok(&["shadow-import", "group.json", "--check-only"]);
    assert!(!s.dir.join("nested").exists());
    assert_eq!(total(&s.json(VERB), "written"), 3);
    assert_eq!(destination(&s, "yard").segments().len(), 2);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(s.dir.join("nested"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
}
