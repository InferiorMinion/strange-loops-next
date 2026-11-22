//! The directory place: a store is the union of its segments.
use loops::{batch, envelope::Env, json::Json, record::Shown, stored, things};
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

#[test]
fn the_writer_view_of_a_held_record_decides_the_batch_checks_against_the_store() {
    let place = Dir::init(&scratch("writer-view")).expect("init");
    let key = Key::generate().expect("key");
    let root = line(&key, "root", &[]);
    let revision = {
        let env = Env {
            kind: "note".into(),
            observer: key.public(),
            emitted: "2026-11-09T00:00:00Z".into(),
            about: vec!["topic:mentioned".into()],
            of: Some(root.id().to_owned()),
            prev: vec![root.id().to_owned()],
            ..Env::default()
        };
        Line::sign(
            &stored::write(env, key::salt().expect("salt"), Json::Null).expect("written"),
            &key,
        )
    };
    place
        .append(&[root.clone(), revision.clone()])
        .expect("append");
    let held = place.read().expect("read");
    assert_eq!(held.writer_view(root.id()), Some(batch::Held::default()));
    assert_eq!(
        held.writer_view(revision.id()),
        Some(batch::Held {
            of: Some(root.id().to_owned()),
            prev: vec![root.id().to_owned()],
        })
    );
    assert_eq!(held.writer_view("r:unknown"), None);
    let hash = loops_place::line::hash;
    let lookup = |id: &str| held.writer_view(id);
    let draft = |of: Option<&str>, prev: &[&str]| batch::Draft {
        label: None,
        env: Env {
            kind: "note".into(),
            observer: key.public(),
            emitted: "2026-11-09T00:00:01Z".into(),
            of: of.map(str::to_owned),
            prev: prev.iter().map(|s| (*s).to_owned()).collect(),
            ..Env::default()
        },
        salt: vec![0; 16],
        payload: Json::Null,
    };
    // A revision of the root is of the root; naming the revision as a thing is not.
    assert!(
        batch::resolve_batch(&hash, &lookup, &[draft(Some(root.id()), &[revision.id()])]).is_ok()
    );
    assert_eq!(
        batch::resolve_batch(&hash, &lookup, &[draft(Some(revision.id()), &[])]),
        Err(batch::Error::OfNotRoot)
    );
    // `about` is a mention: the revision is of the root's thing, not of what it mentions.
    assert_eq!(
        batch::resolve_batch(
            &hash,
            &lookup,
            &[draft(Some("topic:mentioned"), &[revision.id()])]
        ),
        Err(batch::Error::PrevUnrelated)
    );
    let records = held.settled();
    assert_eq!(things::state_in("topic:mentioned", &records).heads, vec![]);
    assert_eq!(
        things::state_in(root.id(), &records)
            .heads
            .iter()
            .map(|r| loops::record::id_text(&r.id))
            .collect::<Vec<_>>(),
        [revision.id()]
    );
}

#[test]
fn indexed_line_keeps_the_first_preferred_form_in_segment_order() {
    let key = Key::generate().expect("key");
    let full = line(&key, "opened", &[]);
    let bare = line(&key, "unopened", &[]).erased();
    let forms = [full.erased(), bare.clone(), full.clone(), full, bare];
    let place = Dir::init(&scratch("indexed-line")).expect("init");
    place.append(&forms).expect("append");
    let held = place.read().expect("read");
    for id in forms.iter().map(Line::id).chain(["absent"]) {
        let mut matches = held.lines().iter().filter(|line| line.id() == id);
        let scanned = matches.next().map(|first| {
            if first.payload().is_some() {
                first
            } else {
                matches
                    .find(|line| line.payload().is_some())
                    .unwrap_or(first)
            }
        });
        assert_eq!(
            held.line(id).map(std::ptr::from_ref),
            scanned.map(std::ptr::from_ref)
        );
    }
}

fn cache_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("loops-cache-{name}-{}", std::process::id()));
    fs::create_dir(&dir).expect("private test root");
    dir
}

fn assert_same_read(place: &Dir, cache: &std::path::Path) {
    let full = place.read().expect("verify all signatures");
    for _ in 0..2 {
        let cached = place.read_with_cache(cache).expect("ordinary read");
        assert_eq!(cached.segments(), full.segments());
        assert_eq!(cached.lines(), full.lines());
        assert_eq!(cached.settled(), full.settled());
        assert_eq!(cached.refused(), full.refused());
        assert_eq!(cached.altered(), full.altered());
    }
}

fn segment_digest(bytes: &[u8]) -> String {
    loops::base32::base32(&loops_place::line::hash(bytes))
}

#[test]
fn only_complete_clean_matching_segments_supply_cache_entries() {
    let root = cache_test_dir("eligibility");
    let place = Dir::init(&root.join(".loops")).expect("store");
    let cache = root.join("cache");
    let list = cache.join("checked-segments-v1");
    let key = Key::generate().expect("key");
    let good = line(&key, "checked", &[]);
    place
        .append(&[good.clone(), good.clone()])
        .expect("segment");
    let path = files(&place).remove(0);
    let whole = fs::read(&path).expect("two lines");
    assert_same_read(&place, &cache);
    let remembered = format!("{}\n", segment_digest(&whole));
    assert_eq!(fs::read_to_string(&list).expect("list"), remembered);
    // A digest hit is insufficient when the name is wrong.
    let wrong = path.with_file_name("wrong.jsonl");
    fs::rename(&path, &wrong).expect("rename");
    assert_same_read(&place, &cache);
    assert_eq!(place.read().expect("full").altered().len(), 1);
    fs::rename(&wrong, &path).expect("restore name");

    // Both torn files hold their first line. Only the original name is altered.
    let cut = &whole[..whole.len() - 1];
    fs::write(&path, cut).expect("truncate");
    assert_same_read(&place, &cache);
    let held = place.read().expect("full");
    assert_eq!(held.lines(), std::slice::from_ref(&good));
    assert_eq!(held.altered().len(), 1);
    let fitting = path.with_file_name(format!("{}.jsonl", segment_digest(cut)));
    fs::rename(&path, &fitting).expect("match truncated bytes");
    assert_same_read(&place, &cache);
    assert_eq!(place.read().expect("full").altered(), Vec::<String>::new());
    assert_eq!(fs::read_to_string(&list).expect("list"), remembered);

    let invalid = format!("{}\n{{}}\n", good.render());
    let bad = path.with_file_name(format!("{}.jsonl", segment_digest(invalid.as_bytes())));
    fs::rename(fitting, &bad).expect("move segment");
    fs::write(&bad, invalid).expect("replace bytes");
    assert_same_read(&place, &cache);
    assert_eq!(place.read().expect("full").refused().len(), 1);
    assert_eq!(fs::read_to_string(&list).expect("list"), remembered);
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn a_signature_change_with_the_same_size_and_time_misses_the_cache() {
    let root = cache_test_dir("bytes");
    let place = Dir::init(&root.join(".loops")).expect("store");
    let cache = root.join("cache");
    let list = cache.join("checked-segments-v1");
    let good = line(&Key::generate().expect("key"), "one", &[]);
    place.append(&[good]).expect("write");
    assert_same_read(&place, &cache);
    let path = files(&place).remove(0);
    let modified = fs::metadata(&path)
        .expect("stat")
        .modified()
        .expect("mtime");
    let mut bytes = fs::read_to_string(&path).expect("line");
    let offset = bytes.find("\"ed25519\":\"").expect("signature") + "\"ed25519\":\"".len();
    let digit = if bytes.as_bytes()[offset] == b'0' {
        "1"
    } else {
        "0"
    };
    bytes.replace_range(offset..=offset, digit);
    fs::write(&path, &bytes).expect("changed signature");
    fs::File::options()
        .write(true)
        .open(&path)
        .expect("open")
        .set_modified(modified)
        .expect("restore mtime");
    assert_same_read(&place, &cache);
    let bad_hash = segment_digest(bytes.as_bytes());
    let fitting = path.with_file_name(format!("{bad_hash}.jsonl"));
    fs::rename(&path, &fitting).expect("matching name");
    assert_same_read(&place, &cache);
    assert_eq!(
        place.read_with_cache(&cache).expect("read").refused()[0].refusal,
        Refusal::SignatureInvalid
    );

    // A different verifier version, or a damaged list with one valid entry,
    // cannot vouch for a signature. All entries of a corrupt list are ignored.
    fs::write(&list, format!("{bad_hash}\n")).expect("test entry");
    fs::rename(&list, cache.join("checked-segments-v0")).expect("older rules");
    assert_same_read(&place, &cache);
    fs::rename(cache.join("checked-segments-v0"), &list).expect("restore");
    fs::write(&list, format!("{bad_hash}\ntruncated")).expect("corrupt list");
    assert_same_read(&place, &cache);
    // A cache hit skips just signature verification: a bad id still refuses.
    bytes = bytes.replace("\"text\":\"one\"", "\"text\":\"two\"");
    let digest = segment_digest(bytes.as_bytes());
    let changed = path.with_file_name(format!("{digest}.jsonl"));
    fs::rename(fitting, &changed).expect("rename");
    fs::write(changed, bytes).expect("changed payload");
    fs::write(&list, format!("{digest}\n")).expect("test entry");
    assert_same_read(&place, &cache);
    assert_eq!(
        place.read().expect("full").refused()[0].refusal,
        Refusal::IdMismatch
    );
    fs::remove_dir_all(root).expect("clean test directory");
}

#[test]
fn a_store_cannot_supply_the_cache_and_cache_io_is_optional() {
    let root = cache_test_dir("location");
    let place = Dir::init(&root.join(".loops")).expect("store");
    place
        .append(&[line(&Key::generate().expect("key"), "one", &[])])
        .expect("write");
    let inside = place.root().join("cache");
    assert_same_read(&place, &inside);
    assert!(!inside.exists());
    let blocked = root.join("ordinary-file");
    fs::write(&blocked, "not a directory").expect("block directory creation");
    assert_same_read(&place, &blocked.join("cache"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let alias = root.join("alias");
        symlink(place.root(), &alias).expect("alias to store");
        assert_same_read(&place, &alias.join("cache"));
        assert!(!inside.exists());
        let readonly = root.join("readonly");
        fs::create_dir(&readonly).expect("directory");
        fs::set_permissions(&readonly, fs::Permissions::from_mode(0o500)).expect("unwritable");
        assert_same_read(&place, &readonly);
        assert_eq!(fs::read_dir(&readonly).expect("directory").count(), 0);
        fs::set_permissions(&readonly, fs::Permissions::from_mode(0o700)).expect("restore");
    }
    fs::remove_dir_all(root).expect("clean test directory");
}
