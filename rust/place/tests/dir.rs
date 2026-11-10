//! The directory place: a store is the union of its segments.
use loops::{envelope::Env, json::Json, record::Shown, stored};
use loops_place::{
    dir::Dir,
    key::{self, Key},
    line::{Line, Refusal},
};
use std::{fs, path::PathBuf};

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("dir-{name}"));
    let _ = fs::remove_dir_all(&dir);
    dir
}
fn line(key: &Key, text: &str, erases: &[&str]) -> Line {
    let env = Env {
        kind: "note".into(),
        observer: key.public(),
        emitted: "2026-11-09T00:00:00Z".into(),
        erases: erases.iter().map(|s| (*s).to_owned()).collect(),
        ..Env::default()
    };
    let payload = Json::Obj(vec![("text".into(), Json::Str(text.into()))]);
    Line::sign(
        &stored::write(env, key::salt().expect("salt"), payload).expect("written"),
        key,
    )
}
fn ids(place: &Dir) -> Vec<String> {
    let mut ids: Vec<_> = place
        .read()
        .expect("read")
        .settled()
        .iter()
        .map(|r| loops::record::id_text(&r.id))
        .collect();
    ids.sort();
    ids
}
fn files(place: &Dir) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in fs::read_dir(place.root().join("segments")).expect("segments") {
        for file in fs::read_dir(dir.expect("entry").path()).expect("key directory") {
            found.push(file.expect("entry").path());
        }
    }
    found.sort();
    found
}

#[test]
fn a_changeset_is_one_closed_segment_per_key_and_writing_it_twice_is_a_no_op() {
    let place = Dir::init(&scratch("changeset")).expect("init");
    let (a, b) = (Key::generate().expect("key"), Key::generate().expect("key"));
    let lines = [
        line(&a, "one", &[]),
        line(&b, "two", &[]),
        line(&a, "three", &[]),
    ];
    let first = place.append(&lines).expect("append");
    assert_eq!(first.len(), 2);
    assert_eq!(place.append(&lines).expect("again"), first);
    assert_eq!(files(&place).len(), 2);
    let held = place.read().expect("read");
    assert_eq!((held.segments(), held.lines().len()), (2, 3));
    assert!(held.refused().is_empty() && held.altered().is_empty());
    assert_eq!(place.append(&[]).expect("nothing"), Vec::<String>::new());
    assert!(Dir::open(&scratch("absent")).is_err());
}

#[test]
fn two_copies_merge_by_adding_files_whatever_the_order() {
    let key = Key::generate().expect("key");
    let (x, y) = (
        Dir::init(&scratch("merge-x")).expect("init"),
        Dir::init(&scratch("merge-y")).expect("init"),
    );
    let shared = line(&key, "shared", &[]);
    x.append(std::slice::from_ref(&shared)).expect("append");
    y.append(std::slice::from_ref(&shared)).expect("append");
    x.append(&[line(&key, "only x", &[])]).expect("append");
    y.append(&[line(&key, "only y", &[])]).expect("append");
    for (from, to) in [(&x, &y), (&y, &x)] {
        for file in files(from) {
            let target = to
                .root()
                .join(file.strip_prefix(from.root()).expect("below the root"));
            if !target.exists() {
                fs::copy(&file, &target).expect("copy");
            }
        }
    }
    assert_eq!(ids(&x).len(), 3);
    assert_eq!(ids(&x), ids(&y));
    assert_eq!(files(&x).len(), 3, "the shared changeset is one file");
}

#[test]
fn purge_drops_an_opening_and_an_eraser_makes_it_erased_everywhere() {
    let place = Dir::init(&scratch("purge")).expect("init");
    let key = Key::generate().expect("key");
    let (secret, kept) = (line(&key, "secret", &[]), line(&key, "kept", &[]));
    place
        .append(&[secret.clone(), kept.clone()])
        .expect("append");
    let shown = |id: &str| {
        let held = place.read().expect("read");
        let record = held
            .settled()
            .into_iter()
            .find(|r| loops::record::id_text(&r.id) == id)
            .expect("held");
        (
            record.shown,
            held.line(id).expect("line").payload().is_some(),
        )
    };
    assert_eq!(place.purge(&[secret.id().to_owned()]).expect("purge"), 1);
    assert_eq!(place.purge(&[secret.id().to_owned()]).expect("again"), 0);
    // A bare commitment with no held eraser is withheld, not erased.
    assert_eq!(shown(secret.id()), (Shown::Withheld, false));
    assert_eq!(shown(kept.id()), (Shown::Here, true));
    let held = place.read().expect("read");
    assert!(held.altered().is_empty() && held.refused().is_empty());
    assert_eq!(held.segments(), 1);
    // A copy that still holds the opening comes back: the opening is held again,
    // and only a held eraser keeps it from being read.
    place.append(std::slice::from_ref(&secret)).expect("append");
    assert_eq!(shown(secret.id()), (Shown::Here, true));
    place
        .append(&[line(&key, "", &[secret.id()])])
        .expect("append");
    assert_eq!(shown(secret.id()), (Shown::Erased, true));
    assert_eq!(place.purge(&[secret.id().to_owned()]).expect("purge"), 1);
    assert_eq!(shown(secret.id()), (Shown::Erased, false));
    assert_eq!(
        place.read().expect("read").ids_from(&secret.id()[2..9]),
        [secret.id()]
    );
}

#[test]
fn a_torn_last_line_is_skipped_and_a_changed_segment_is_named() {
    let place = Dir::init(&scratch("torn")).expect("init");
    let key = Key::generate().expect("key");
    let (one, two) = (line(&key, "one", &[]), line(&key, "two", &[]));
    place.append(&[one.clone(), two.clone()]).expect("append");
    let file = files(&place).remove(0);
    let whole = fs::read_to_string(&file).expect("segment");
    // A write that stopped inside the second line.
    fs::write(&file, &whole[..whole.len() - 20]).expect("truncate");
    let held = place.read().expect("read");
    assert_eq!(held.lines(), std::slice::from_ref(&one));
    assert_eq!(held.refused(), []);
    assert_eq!(held.altered().len(), 1);
    // An edit inside a line is refused by the line's own check, and the segment is named.
    fs::write(&file, whole.replace("\"text\":\"one\"", "\"text\":\"won\"")).expect("edit");
    let held = place.read().expect("read");
    assert_eq!(held.lines(), [two]);
    assert_eq!(held.altered().len(), 1);
    assert_eq!(
        held.refused()
            .iter()
            .map(|r| (r.line, r.refusal))
            .collect::<Vec<_>>(),
        [(1, Refusal::IdMismatch)]
    );
}
