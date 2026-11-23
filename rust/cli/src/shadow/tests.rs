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
fn fixture(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("shadow-recovery-{label}-{}", std::process::id()));
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
use loops_place::line::Line;

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
        };
        assert!(matches!(invoke(&dir, &first), Err(Fail::System(_))));
        let journal = dir.join("private/journal.jsonl");
        let j = success(Journal::read(&journal));
        assert_eq!(
            j.seeds.len(),
            if point == Point::BeforeReserve { 0 } else { 3 }
        );
        assert_eq!(
            lines(&dir).len(),
            if point == Point::Appended { 3 } else { 0 }
        );
        assert_eq!(j.done.len(), 0);
        let mut retry = fixed();
        retry.key = || panic!("must reuse key");
        if point != Point::BeforeReserve {
            retry.seed = |_, _| panic!("must replay the reservation");
        }
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
