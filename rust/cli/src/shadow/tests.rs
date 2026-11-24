//! Injected only in this test build; the product has no fault or seed override.
use super::*;
use std::io::Write as _;

fn success<T>(r: Result<T, Fail>) -> T {
    match r {
        Ok(v) => v,
        Err(Fail::Refused(s) | Fail::Usage(s) | Fail::System(s)) => panic!("{s}"),
        Err(Fail::TickNotSame | Fail::Closed) => panic!("unexpected CLI exit"),
    }
}
fn fixture_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("shadow-recovery-{label}-{}", std::process::id()))
}
fn fixture(label: &str) -> PathBuf {
    let root = fixture_root(label);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("workshop/.loops/segments")).unwrap();
    fs::write(root.join("group.json"),r#"{"state":"private","sources":[{"name":"workshop","facts":"cut","map":"map","destination":"workshop"}]}"#).unwrap();
    fs::write(
        root.join("cut"),
        include_bytes!("../../tests/fixtures/shadow/workshop.facts.jsonl"),
    )
    .unwrap();
    fs::write(
        root.join("map"),
        include_bytes!("../../tests/fixtures/shadow/workshop.map.json"),
    )
    .unwrap();
    root
}
fn fixed() -> Runtime {
    Runtime {
        key: || Ok(Key::from_text(&"0".repeat(52)).unwrap()),
        seed: |old, fingerprint| {
            Ok(Seed {
                fingerprint,
                salt: hash(old.as_bytes())[..16].to_vec(),
                emitted: "2026-11-14T00:00:00Z".into(),
            })
        },
        at: |_| Ok(()),
        verified: drop,
    }
}
fn invoke(root: &Path, r: &Runtime) -> Result<(), Fail> {
    import(
        &root.join("group.json"),
        Unnamed::Omit,
        Mode::Write,
        r,
        &mut Report::default(),
    )
}
fn lines(root: &Path) -> Vec<String> {
    Dir::open(&root.join("workshop/.loops"))
        .unwrap()
        .read()
        .unwrap()
        .lines()
        .iter()
        .map(Line::render)
        .collect()
}

#[test]
fn interruptions_recover_the_same_signed_lines() {
    fn stop() -> Result<(), Fail> {
        Err(Fail::System("test interruption".into()))
    }
    let base = fixture("uninterrupted");
    success(invoke(&base, &fixed()));
    let expected = lines(&base);
    assert_eq!(expected.len(), 3);
    for point in [Point::BeforeReserve, Point::Reserved, Point::Appended] {
        let dir = fixture(&format!("{point:?}"));
        let mut first = fixed();
        first.at = match point {
            Point::BeforeReserve => |p| {
                if p == Point::BeforeReserve {
                    stop()
                } else {
                    Ok(())
                }
            },
            Point::Reserved => |p| if p == Point::Reserved { stop() } else { Ok(()) },
            Point::Appended => |p| if p == Point::Appended { stop() } else { Ok(()) },
            _ => unreachable!(),
        };
        assert!(matches!(invoke(&dir, &first), Err(Fail::System(_))));
        let journal = dir.join("private/journal.jsonl");
        let j = success(Journal::read(&journal));
        assert_eq!(j.seeds.len(), 3);
        assert_eq!(
            lines(&dir).len(),
            if point == Point::Appended { 3 } else { 0 }
        );
        assert_eq!(j.done.len(), 0);
        let mut retry = fixed();
        retry.key = || panic!("must reuse key");
        retry.seed = |_, _| panic!("must replay the reservation");
        success(invoke(&dir, &retry));
        assert_eq!(lines(&dir), expected);
        assert_eq!(success(Journal::read(&journal)).done.len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn importer_lock_and_reservation_survive_separate_invocations() {
    let root = fixture("lock");
    success(invoke(&root, &fixed()));
    let state = root.join("private");
    let lock = success(journal::lock(&state, Mode::Check)).unwrap();
    assert!(matches!(invoke(&root,&fixed()),Err(Fail::Refused(s)) if s.contains("lock")));
    drop(lock);
    let before = lines(&root);
    let place = Dir::open(&root.join("workshop/.loops")).unwrap();
    let held = place.read().unwrap();
    let id = held.lines()[0].id().to_owned();
    place.purge(std::slice::from_ref(&id)).unwrap();
    success(invoke(&root, &fixed()));
    assert!(place.read().unwrap().line(&id).unwrap().payload().is_none());
    // A truly missing reservation, even one marked complete, can be replayed.
    for dir in fs::read_dir(place.root().join("segments")).unwrap() {
        for file in fs::read_dir(dir.unwrap().path()).unwrap() {
            fs::remove_file(file.unwrap().path()).unwrap();
        }
    }
    success(invoke(&root, &fixed()));
    assert_eq!(lines(&root), before);
    fs::OpenOptions::new()
        .append(true)
        .open(state.join("key"))
        .unwrap()
        .write_all(b"bad")
        .unwrap();
    assert!(matches!(invoke(&root, &fixed()), Err(Fail::Refused(_))));
    fs::remove_dir_all(root).unwrap();
}

fn group_fixture(label: &str, order: [&str; 2]) -> PathBuf {
    use loops::json::quote;
    use std::fmt::Write as _;

    let root = fixture(label);
    let mut entries = Vec::new();
    for name in order {
        fs::create_dir_all(root.join(name).join(".loops/segments")).unwrap();
        let map =
            include_str!("../../tests/fixtures/shadow/workshop.map.json").replace("workshop", name);
        fs::write(root.join(format!("{name}.map")), map).unwrap();
        let header = include_str!("../../tests/fixtures/shadow/workshop.facts.jsonl")
            .lines()
            .next()
            .unwrap()
            .replace("workshop", name)
            .replace("arrival", "rowid");
        let mut bytes = format!("{header}\n");
        let mut digest = hash(bytes.as_bytes());
        for (i, (id, kind, payload)) in [
            ("potter", "thread", r#"{"name":"potter"}"#),
            ("fire", "change", "{}"),
            ("kiln-end", "change", "{}"),
            ("kiln-middle", "change", r#"{"ref":"kiln-end"}"#),
            ("kiln-first", "change", r#"{"ref":"kiln-middle"}"#),
            ("no-key", "thread", "{}"),
            ("note", "moved", "{}"),
            ("defect", "moved", "[]"),
            ("later", "thread", r#"{"name":"potter","heat":"hot"}"#),
        ]
        .iter()
        .enumerate()
        {
            // The same "later" fact has different predecessors in the two stores.
            if (name == "yard" && matches!(*id, "potter" | "kiln-middle"))
                || (name == "workshop" && matches!(*id, "kiln-end" | "kiln-first"))
            {
                continue;
            }
            let start = bytes.len();
            writeln!(bytes, r#"{{"id":"{id}","kind":"{kind}","ts_bits":"3ff8000000000000","observer":"potter","origin":"","payload":{},"signature":null,"coordinate":["{i}"]}}"#, quote(payload)).unwrap();
            digest.extend_from_slice(&bytes.as_bytes()[start..]);
            digest = hash(&digest);
        }
        let sum = digest.iter().fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").unwrap();
            hex
        });
        let count = bytes.lines().count() - 1;
        writeln!(bytes, r#"{{"count":"{count}","digest":"{sum}"}}"#).unwrap();
        fs::write(root.join(format!("{name}.facts")), bytes).unwrap();
        entries.push(format!(r#"{{"name":"{name}","map":"{name}.map","facts":"{name}.facts","destination":"{name}"}}"#));
    }
    fs::write(
        root.join("group.json"),
        format!(r#"{{"state":"private","sources":[{}]}}"#, entries.join(",")),
    )
    .unwrap();
    root
}

#[test]
fn collected_occurrences_classify_each_destination_before_preparation() {
    use mapping::{Fate, Target};

    let root = group_fixture("fates", ["workshop", "yard"]);
    for unnamed in [Unnamed::Omit, Unnamed::Roots] {
        let group = success(Group::read(&root.join("group.json"), unnamed));
        let (index, _) = success(plan::Index::collect(
            &group.inputs,
            unnamed,
            &Journal::default(),
            &mut Report::for_inputs(&group.inputs),
        ));
        for source in 0..2 {
            let fate = |id| &index.occurrence(id, source).fate;
            for (id, target) in [
                ("later", Target::Keyed("thread:potter".into())),
                ("fire", Target::Collected),
                ("no-key", Target::MissingKey),
            ] {
                assert!(matches!(fate(id), Fate::Included(Ok(row)) if row.target == target));
            }
            if unnamed == Unnamed::Omit {
                assert!(matches!(fate("note"), Fate::Omitted { defect: None }));
                assert!(
                    matches!(fate("defect"), Fate::Omitted { defect: Some(e) } if e.code() == "shadow-source-invalid")
                );
            } else {
                assert!(
                    matches!(fate("note"), Fate::Included(Ok(row)) if row.target == Target::Unnamed)
                );
                assert!(
                    matches!(fate("defect"), Fate::Included(Err(e)) if e.code() == "shadow-source-invalid")
                );
            }
        }
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn checked_and_published_payload_bytes_agree_after_removal() {
    use std::cell::RefCell;
    thread_local! { static CHECKED: RefCell<Vec<Line>> = const { RefCell::new(Vec::new()) }; }

    let root = group_fixture("checked-payload", ["workshop", "yard"]);
    let mut runtime = fixed();
    runtime.verified = |lines| CHECKED.with_borrow_mut(|checked| checked.extend(lines));
    success(import(
        &root.join("group.json"),
        Unnamed::Omit,
        Mode::Check,
        &runtime,
        &mut Report::default(),
    ));
    success(invoke(&root, &fixed()));

    // The workshop root kiln-middle originally held {"ref":"kiln-end"}.
    let journal = success(Journal::read(&root.join("private/journal.jsonl")));
    let (id, _) = &journal.records["workshop"]["kiln-middle"];
    let checked = CHECKED
        .take()
        .into_iter()
        .find(|line| line.id() == id)
        .unwrap();
    let bytes = checked.payload().unwrap().canon().into_bytes();
    assert_eq!(bytes, b"{}");
    let held = Dir::open(&root.join("workshop/.loops"))
        .unwrap()
        .read()
        .unwrap();
    assert_eq!(
        held.line(id).unwrap().payload().unwrap().canon().as_bytes(),
        bytes
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_order_preserves_segment_bytes_and_each_destination_is_read_once() {
    use crate::{count, values::member};
    use std::collections::BTreeMap;

    let run = |label, order| {
        let root = group_fixture(label, order);
        for mode in [Mode::Check, Mode::Write, Mode::Write] {
            let mut report = Report::default();
            success(import(
                &root.join("group.json"),
                Unnamed::Omit,
                mode,
                &fixed(),
                &mut report,
            ));
            let json = report.json();
            let sources = member(&json, "sources").unwrap();
            for name in order {
                let source = member(sources, name).unwrap();
                assert_eq!(member(source, "destination_reads"), Some(&count(1)));
            }
        }
        let mut segments = BTreeMap::new();
        for name in order {
            for signer in fs::read_dir(root.join(name).join(".loops/segments")).unwrap() {
                for file in fs::read_dir(signer.unwrap().path()).unwrap() {
                    let path = file.unwrap().path();
                    segments.insert(
                        path.strip_prefix(&root).unwrap().to_owned(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }
        assert_eq!(segments.len(), 2);
        segments.insert(
            "group.ids.json".into(),
            fs::read(root.join("group.ids.json")).unwrap(),
        );
        fs::remove_dir_all(root).unwrap();
        segments
    };
    assert_eq!(
        run("forward", ["workshop", "yard"]),
        run("reverse", ["yard", "workshop"])
    );
}

#[test]
fn preparation_rereads_the_previously_indexed_remote_cut() {
    let root = group_fixture("reread", ["workshop", "yard"]);
    let mut runtime = fixed();
    runtime.seed = |old, fp| {
        // The first workshop reservation runs after both cuts were collected.
        if old == "potter" {
            let path = fixture_root("reread").join("yard.facts");
            let cut = fs::read_to_string(&path).unwrap();
            fs::write(path, cut.replace("hot", "wet")).unwrap();
        }
        (fixed().seed)(old, fp)
    };
    assert!(
        matches!(invoke(&root, &runtime), Err(Fail::Refused(s)) if s.contains("export changed after preflight"))
    );
    assert!(!root.join("private").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cross_store_recovery_finishes_the_same_map_and_segments() {
    use std::{cell::Cell, collections::BTreeMap};
    thread_local! { static STOP: Cell<Option<Point>> = const { Cell::new(None) }; }
    let outputs = |root: &Path| {
        let mut bytes = BTreeMap::new();
        for store in ["workshop", "yard"] {
            for signer in fs::read_dir(root.join(store).join(".loops/segments")).unwrap() {
                for entry in fs::read_dir(signer.unwrap().path()).unwrap() {
                    let path = entry.unwrap().path();
                    bytes.insert(
                        path.strip_prefix(root).unwrap().to_owned(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }
        bytes.insert(
            "group.ids.json".into(),
            fs::read(root.join("group.ids.json")).unwrap(),
        );
        bytes
    };
    let base = group_fixture("durable-base", ["workshop", "yard"]);
    success(invoke(&base, &fixed()));
    let expected = outputs(&base);
    for point in [
        Point::BeforeSeeds,
        Point::SeedsSynced,
        Point::Reserved,
        Point::Appended,
        Point::BeforeMapRename,
    ] {
        let root = group_fixture(&format!("durable-{point:?}"), ["workshop", "yard"]);
        STOP.with(|s| s.set(Some(point)));
        let mut first = fixed();
        first.at = |p| {
            if STOP.with(Cell::get) == Some(p) {
                Err(Fail::System("interrupted".into()))
            } else {
                Ok(())
            }
        };
        assert!(matches!(invoke(&root, &first), Err(Fail::System(_))));
        let log = root.join("private/journal.jsonl");
        let j = success(Journal::read(&log));
        assert_eq!(
            j.seeds.contains_key("kiln-end"),
            point != Point::BeforeSeeds
        );
        if point == Point::Reserved {
            assert!(!j.records.contains_key("yard"));
        }
        assert!(!root.join("group.ids.json").exists());
        let prior = fs::read(&log).unwrap();
        success(import(
            &root.join("group.json"),
            Unnamed::Omit,
            Mode::Check,
            &fixed(),
            &mut Report::default(),
        ));
        assert_eq!(prior, fs::read(&log).unwrap());
        assert!(!root.join("group.ids.json").exists());
        let mut retry = fixed();
        if point != Point::BeforeSeeds {
            retry.seed = |_, _| panic!("all group seeds are durable");
        }
        success(invoke(&root, &retry));
        assert_eq!(outputs(&root), expected);
        fs::remove_dir_all(root).unwrap();
    }
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn interrupted_seed_frames_resume_with_only_the_remaining_ids() {
    let root = fixture("seed-frames");
    let log = root.join("seeds.jsonl");
    let mut j = Journal::default();
    let mut file = success(j.writer(&log));
    success(j.append(
        &mut file,
        &event("v2", [text("potter"), Json::Arr(Vec::new())]),
    ));
    let mut ids = plan::Identities::from(&j);
    for n in 0..1001 {
        let old = format!("potter-{n:04}");
        ids.seeds
            .insert(old.clone(), success((fixed().seed)(&old, old.clone())));
    }
    let mut first = fixed();
    first.at = |p| {
        if p == Point::SeedsSynced {
            Err(Fail::System("stop".into()))
        } else {
            Ok(())
        }
    };
    assert!(matches!(
        save_seeds(&ids, &mut j, &mut file, &first),
        Err(Fail::System(_))
    ));
    let mut resumed = success(Journal::read(&log));
    assert_eq!(resumed.seeds.len(), 1000);
    assert_eq!(resumed.batches.len(), 0);
    success(save_seeds(&ids, &mut resumed, &mut file, &fixed()));
    assert_eq!(success(Journal::read(&log)).seeds, ids.seeds);
    fs::remove_dir_all(root).unwrap();
}
