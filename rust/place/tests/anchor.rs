//! The anchor rule, end to end: a history written as `loops emit` writes it
//! is anchored, and each thing's live records, with the records that erase
//! them, answer every slot that follows `keys` as the whole store does.
//!
//! Each emit goes the single-record path's way: `fill::of`, `fill::anchor`,
//! the batch writer against what the writer holds, then a signature. Each
//! states its own `sets`, as `--sets`, `--sets-nothing` or `--whole` does, so
//! `fill::sets` fills nothing in. An erasure goes `loops erase`'s way, a root
//! with `erases` and nothing filled in.
use loops::{
    batch::{self, Draft},
    envelope::Env,
    json::Json,
    live,
    record::{Record, Shown, id_text},
    spec::Spec,
    things,
};
use loops_place::{
    dir::Dir,
    fill::{self, About},
    key::{self, Key},
    line::{Line, hash},
};
use std::{collections::HashSet, fs, path::PathBuf};

/// splitmix64: a fixed seed gives the same history on every machine.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// A number below `n`.
    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).expect("n fits")).expect("below n")
    }
    fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[self.below(xs.len())]
    }
}

const THINGS: usize = 6;
const KEYS: [&str; 4] = ["status", "title", "n", "owner"];

/// One writer's holding: the lines it has, and the records they read as.
struct Replica {
    key: Key,
    lines: Vec<Line>,
    records: Vec<Record>,
}
impl Replica {
    fn new() -> Self {
        Self {
            key: Key::generate().expect("key"),
            lines: Vec::new(),
            records: Vec::new(),
        }
    }
    /// The store as a reader sees it: settled, then admitted.
    fn realm(&self) -> Vec<Record> {
        things::admit(&things::settle(&self.records), |_, _| true)
    }
    fn hold(&mut self, line: Line) {
        if !self.lines.iter().any(|l| l.id() == line.id()) {
            self.records.push(line.record());
            self.lines.push(line);
        }
    }
    fn write(&mut self, draft: Draft, about: Option<About>) -> String {
        let view = |id: &str| {
            self.lines
                .iter()
                .find(|l| l.id() == id)
                .map(|l| batch::Held {
                    of: l.env().of.clone(),
                    prev: l.env().prev.clone(),
                })
        };
        let draft = match about {
            Some(about) => {
                let draft = fill::of(&view, draft).expect("of is filled in");
                fill::anchor(&self.realm(), draft, about)
            }
            None => draft,
        };
        let written = batch::resolve_batch(&hash, &view, &[draft]).expect("written");
        let line = Line::sign(&written[0], &self.key);
        let id = line.id().to_owned();
        self.hold(line);
        id
    }
}

fn emitted(step: usize) -> String {
    format!(
        "2026-11-14T{:02}:{:02}:{:02}Z",
        step / 3600,
        step / 60 % 60,
        step % 60
    )
}

/// One random step on `w`: an emit of a thing, or an erasure.
fn step(rng: &mut Rng, w: &mut Replica, at: usize) {
    let thing = format!("task:{}", rng.below(THINGS));
    let held: Vec<String> = w
        .records
        .iter()
        .filter(|r| r.is_of(&thing))
        .map(|r| id_text(&r.id))
        .collect();
    let roll = rng.below(100);
    if roll < 6 && !held.is_empty() {
        let target = held[rng.below(held.len())].clone();
        let draft = Draft {
            label: None,
            env: Env {
                kind: "erasure".into(),
                observer: w.key.public(),
                emitted: emitted(at),
                erases: vec![target],
                ..Env::default()
            },
            salt: key::salt().expect("salt"),
            payload: Json::Obj(Vec::new()),
        };
        w.write(draft, None);
        return;
    }
    let mut env = Env {
        kind: "task".into(),
        observer: w.key.public(),
        emitted: emitted(at),
        of: Some(thing),
        ..Env::default()
    };
    // Whole-read emits, pure joins, and emits that set one or two keys.
    let sets: Option<Vec<String>> = match roll {
        _ if held.is_empty() || roll < 16 => None,
        16..24 => Some(Vec::new()),
        _ => {
            let mut ks = vec![rng.pick(&KEYS).to_owned()];
            if rng.below(3) == 0 {
                ks.push(rng.pick(&KEYS).to_owned());
            }
            Some(ks)
        }
    };
    let mut payload = Vec::new();
    for k in KEYS {
        // A declared key is now and then left out, and an undeclared one written.
        let declared = sets.as_ref().is_none_or(|ks| ks.iter().any(|x| x == k));
        if declared == (rng.below(8) != 0) {
            let value = match k {
                "n" => Json::Int(i64::try_from(rng.below(6)).expect("small").into()),
                "status" => Json::Str(rng.pick(&["open", "done", "held"]).into()),
                _ => Json::Str(rng.pick(&["a", "b", "c"]).into()),
            };
            payload.push((k.to_owned(), value));
        }
    }
    env.sets = sets;
    // Now and then an explicit `prev` naming any record of the thing.
    if !held.is_empty() && rng.below(10) == 0 {
        env.prev.push(held[rng.below(held.len())].clone());
    }
    let about = match rng.below(10) {
        0 | 1 => {
            env.about
                .push(format!("project:{}", rng.pick(&["x", "y", "z"])));
            About::Stated
        }
        2 => About::Stated,
        _ if held.is_empty() => {
            env.about.push("project:x".into());
            About::Stated
        }
        _ => About::Copied,
    };
    let draft = Draft {
        label: None,
        env,
        salt: key::salt().expect("salt"),
        payload: Json::Obj(payload),
    };
    w.write(draft, Some(about));
}

/// Every record above a record live in `realm` names it in `prev`.
fn assert_anchored(realm: &[Record]) {
    for t in things::things_of(realm) {
        let rs = live::records_of(&t, realm);
        for x in rs.iter().filter(|x| live::live(&rs, x)) {
            for y in live::above(&rs, x) {
                assert!(
                    y.prev.contains(&x.id),
                    "{} is above live {} and does not name it",
                    id_text(&y.id),
                    id_text(&x.id)
                );
            }
        }
    }
}

/// Each thing's live records, and the records that erase them.
fn slice(w: &Replica) -> Vec<Line> {
    let realm = w.realm();
    let mut keep: HashSet<String> = things::things_of(&realm)
        .iter()
        .flat_map(|t| live::live_in(&realm, t))
        .map(|r| id_text(&r.id))
        .collect();
    let erasers: Vec<String> = w
        .records
        .iter()
        .filter(|r| r.erases.iter().any(|e| keep.contains(&id_text(e))))
        .map(|r| id_text(&r.id))
        .collect();
    keep.extend(erasers);
    w.lines
        .iter()
        .filter(|l| keep.contains(l.id()))
        .cloned()
        .collect()
}

const SPEC: &str = r#"{"vertex":"tasks","from":[{"store":".","kind":["task"]}],"fold":{
  "state":{"latest-by":"of","follow":"keys"},
  "open":{"latest-by":"of","follow":"keys","where":{"status":{"is":"open"}}},
  "tasks":{"count":true,"follow":"keys"},
  "open-tasks":{"count":true,"follow":"keys","where":{"status":{"is":"open"}}},
  "in-x":{"count":true,"follow":"keys","where":{"env.about":{"is":"project:x"}}},
  "n":{"sum":"n","follow":"keys"},
  "least":{"min":"n","follow":"keys"},
  "most":{"max":"n","follow":"keys","where":{"title":{"is":"a"}}},
  "live":{"collect":true,"follow":"keys"},
  "first":{"collect":true,"follow":"keys","limit":3},
  "per-status":{"count":true,"follow":"keys","by":"status"},
  "per-project":{"count":true,"follow":"keys","by":"env.about:project"},
  "n-per-project":{"sum":"n","follow":"keys","by":"env.about:project"},
  "live-per-title":{"collect":true,"follow":"keys","by":"title"},
  "owners-per-project":{"collect":true,"follow":"keys","by":"env.about:project","where":{"owner":{"present":true}}}}}"#;

fn answer(records: &[Record]) -> String {
    let spec = Spec::of_json(&loops_eval::parse_json(SPEC).expect("JSON")).expect("a spec");
    loops_eval::slots_json(spec.run(&things::settle(records))).canon()
}

/// The slice answers every keyed slot as the whole holding does.
fn assert_slice_reads(w: &Replica, when: &str) {
    let part: Vec<Record> = slice(w).iter().map(Line::record).collect();
    assert!(part.len() <= w.records.len());
    assert_eq!(answer(&part), answer(&w.records), "{when}");
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("anchor-{name}"));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn one_writers_live_records_read_as_the_whole_store() {
    let mut rng = Rng(0x00d8_c0a7_c4e5_1157);
    let mut w = Replica::new();
    for at in 0..300 {
        step(&mut rng, &mut w, at);
        if at % 25 == 24 {
            assert_anchored(&w.realm());
            assert_slice_reads(&w, &format!("after {} steps", at + 1));
        }
    }
    // The history erases some live records, and the slice is much smaller.
    let realm = w.realm();
    assert!(
        realm
            .iter()
            .any(|r| r.shown != Shown::Here && live::live_in(&realm, &r.thing()).contains(r))
    );
    assert!(slice(&w).len() * 3 < w.lines.len());
    // The same through two stores on disk: the whole, and the slice alone.
    let (whole, part) = (
        Dir::init(&scratch("whole")).expect("init"),
        Dir::init(&scratch("slice")).expect("init"),
    );
    whole.append(&w.lines).expect("append");
    part.append(&slice(&w)).expect("append");
    let (whole, part) = (whole.read().expect("read"), part.read().expect("read"));
    assert!(part.lines().len() < whole.lines().len());
    assert_eq!(answer(&part.settled()), answer(&whole.settled()));
}

#[test]
fn two_replicas_that_merge_read_their_slices_as_their_holdings() {
    let mut rng = Rng(0x00d8_c0a7_c4e5_1158);
    let (mut a, mut b) = (Replica::new(), Replica::new());
    for at in 0..300 {
        let w = if rng.below(2) == 0 { &mut a } else { &mut b };
        step(&mut rng, w, at);
        // Now and then the two merge both ways, so each holds what the
        // writers of its records held.
        if rng.below(12) == 0 {
            for line in a.lines.clone() {
                b.hold(line);
            }
            for line in b.lines.clone() {
                a.hold(line);
            }
        }
        if at % 25 == 24 {
            for (w, name) in [(&a, "a"), (&b, "b")] {
                assert_anchored(&w.realm());
                assert_slice_reads(w, &format!("{name} after {} steps", at + 1));
            }
        }
    }
    // The union of the two, however far apart they were, is anchored too.
    let mut merged = Replica::new();
    for line in a.lines.iter().chain(&b.lines) {
        merged.hold(line.clone());
    }
    // The two forked: some thing has more than one head.
    let realm = merged.realm();
    assert!(
        things::things_of(&realm)
            .iter()
            .any(|t| things::state_in(t, &realm).heads.len() > 1)
    );
    assert_anchored(&realm);
    assert_slice_reads(&merged, "merged");
}
