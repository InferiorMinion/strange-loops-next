//! `loops`: emit, read and fold for one person, over a `.loops/` directory.
//!
//! The verbs follow the design's CLI. Exit codes: 0 ok, 1 refused (with its
//! code), 2 a usage error, 3 the file system or the system's randomness failed.
//! A verb that answers from the store's records writes its answer from the
//! lines that check, then is refused `store-not-clean` when a line or a
//! segment did not.
mod cut;
mod draft;
mod ls;
mod plain;
mod registry;
mod shadow;
mod values;

use loops::{
    batch, envelope,
    json::Json,
    record::{Record, Shown, id_text},
    spec::Spec,
    spec_hash::Rules,
    things,
    tick::read_tick_members,
    tick_check::{TickAnswer, TickSubject, check_tick, tick_open},
};
use loops_place::{
    clock,
    dir::{Dir, Held},
    fill,
    key::Key,
    line::{Line, hash},
};
use plain::Format;
use std::{
    cmp::Ordering,
    env, fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};
use values::View;

const USAGE: &str = "\
usage: loops [-C <dir>] <verb> ...

  init                          make .loops/ here
  emit <kind> [field=text | field:=json]... [\"message\"]
       [--of <thing>] [--about <thing>]... [--no-about] [--prev <id>]...
       [--erases <id>]... [--occurred <when> | --no-occurred] [--under <id>]
       [--sets <key>]... [--sets-nothing] [--whole]
       [--base <tick-id>] [--base-id <id>]...
                                -- ends options; fields and the final message may start with -
                                a revision without --of is of what it replaces
                                by default the writer claims to have read everything held
                                --base selects a tick's covers
                                each --base-id adds a record to the selection
                                include stated prev targets, then follow held prev within the thing
                                add the thing's live records to prev and copy its heads' about
                                copy occurred when every head states the same text
                                the about and occurred flags override copying
                                for a thing held in that view, set the payload keys unless stated
  emit - [--fill [--base <tick-id>] [--base-id <id>]...]
                                read one JSONL changeset from stdin
                                --fill uses one base plus earlier prepared lines
  --version                     print the strange-loops-next version
  ls [--plain]                  list kinds and their record counts
  ls [--things | <prefix>] [--fields a,b,c] [--plain]
                                list things by literal prefix or project their live fields
  get <id>                      one record
  of <thing> [--plain]          a thing's values, each key's from its live
                                setters; its about, its heads, and how many
                                records are of it
                                conflict previews shorten long values; loops of without --plain shows them whole
                                --plain is for reading and may change; scripts use the JSON
  read <vertex> [--store <name>=<dir>]... [--values] [--plain]
                                fold .loops/vertices/<vertex>.json
  fold <spec.json | -> [--store <name>=<dir>]... [--values] [--plain]
                                fold a spec from a file or stdin
                                read and fold run every source over the union of all stores named in the spec
                                bind names with --store (paths relative to -C or the working directory),
                                then stores.json at the current store root or its parent,
                                then $XDG_CONFIG_HOME/loops/stores.json or ~/.config/loops/stores.json
                                only . defaults to the current store, when there is one
                                --store and either list can override that default
                                list paths are relative to the list; an unknown name refuses the read
                                --plain shows slots in the spec's written order
                                plain listings and folds use unique record ID prefixes (8 or more digits)
                                keyed things show their headline without --values
                                --values adds stored envelopes to listed heads,
                                held payloads to collect and prev entries,
                                and about and values to keyed things
  tick <vertex | spec-file | -> [--as-of <time> | --cover <id> ...] [emit envelope flags]
                                default to everything held
                                --as-of selects records emitted strictly before the instant or date’s start
                                repeat --cover to select records by id or unique prefix
                                both cuts include all held prev ancestors
                                a base selects only the tick's envelope anchor
                                a final -- is allowed; no arguments may follow it
  tick --check <id>             check a tick against this store and print the verdict and reports
  erase <id>... [\"why\"]         emit an erasure, and drop the openings held here
  export [--kind <k>] [--of <thing>] [--about <thing>]   stored lines, oldest first
  import [<file> | -]           add stored lines; a store is the union
  shadow-import <group.json> [--include-unnamed] [--check-only]
                                copy legacy exports into dedicated shadow stores
                                include-unnamed imports unmapped kinds as roots
                                check-only preflights the group without writing
  check                         verify every segment and line
  whoami                        the key and observer this machine writes as
  whoami --init                 make the signing key if needed and print its ID";

/// A non-success verdict, a failure, or a closed output stream.
enum Fail {
    Usage(String),
    Refused(String),
    System(String),
    /// The tick verdict was printed and calls for exit 1.
    TickNotSame,
    /// Whoever was reading our answer stopped.
    Closed,
}
impl From<io::Error> for Fail {
    fn from(error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::BrokenPipe {
            // The reader of our answer left, as `head` does. That is its choice.
            Self::Closed
        } else {
            Self::System(error.to_string())
        }
    }
}
fn refused(code: impl Into<String>) -> Fail {
    Fail::Refused(code.into())
}
fn usage(message: impl Into<String>) -> Fail {
    Fail::Usage(message.into())
}

fn object(members: Vec<(&str, Json)>) -> Json {
    Json::Obj(members.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn text(s: impl Into<String>) -> Json {
    Json::Str(s.into())
}
fn count(n: usize) -> Json {
    Json::Int(n.to_string().parse().expect("a count is an integer"))
}
fn say(value: &str) -> Result<(), Fail> {
    let mut out = io::stdout().lock();
    writeln!(out, "{value}")?;
    Ok(())
}

/// What is wrong with a store that does not check, or `None` when it checks:
/// the code, and how many lines were refused and segments altered.
fn unclean(held: &Held) -> Option<String> {
    let (lines, segments) = (held.refused().len(), held.altered().len());
    if lines == 0 && segments == 0 {
        return None;
    }
    let plural = |n: usize, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
    Some(format!(
        "store-not-clean: {} refused, {} altered; `loops check` lists them",
        plural(lines, "line"),
        plural(segments, "segment")
    ))
}
/// The end of a verb that answers from the store: its answer, written from
/// the lines that checked, then `store-not-clean` when the store does not
/// check. The verb's own refusal or failure stands, and a reader that left
/// does not hide an unclean store.
fn answered(held: &Held, answer: Result<(), Fail>) -> Result<(), Fail> {
    finish_answer(answer, unclean(held))
}

fn finish_answer(answer: Result<(), Fail>, why: Option<String>) -> Result<(), Fail> {
    match answer {
        Ok(()) | Err(Fail::Closed) => why.map_or(answer, |why| Err(refused(why))),
        Err(fail) => Err(fail),
    }
}
/// A writer that reads the store to fill in and check what it writes warns
/// when the store does not check, and goes on.
#[expect(
    clippy::print_stderr,
    reason = "An unclean store is said beside a writer's answer."
)]
fn warn_unclean(held: &Held) {
    if let Some(why) = unclean(held) {
        eprintln!("warning: {why}");
    }
}

/// CLI reads can reuse the user's verification cache. The underlying
/// directory always verifies signatures when read directly by `loops check`.
struct Store {
    dir: Dir,
}

impl Store {
    fn read(&self) -> io::Result<Held> {
        let cache = env::var_os("LOOPS_CACHE_DIR")
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("XDG_CACHE_HOME")
                    .map(PathBuf::from)
                    .filter(|p| p.is_absolute())
                    .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
                    .map(|dir| dir.join("loops"))
            });
        match cache {
            Some(dir) => self.dir.read_with_cache(&dir),
            None => self.dir.read(),
        }
    }
}

/// Finds the nearest `.loops/` at or above `start`, as git finds `.git`.
fn current_root(start: &Path) -> Option<&Path> {
    start
        .ancestors()
        .find(|dir| dir.join(".loops/segments").is_dir())
}
fn no_store(start: &Path) -> Fail {
    usage(format!(
        "no .loops/ at or above {}; run `loops init`",
        start.display()
    ))
}
fn find_store(start: &Path) -> Result<Store, Fail> {
    let start = fs::canonicalize(start)?;
    let root = current_root(&start).ok_or_else(|| no_store(&start))?;
    Ok(Store {
        dir: Dir::open(&root.join(".loops"))?,
    })
}

fn key_file() -> Result<PathBuf, Fail> {
    if let Some(path) = env::var_os("LOOPS_KEY_FILE") {
        return Ok(path.into());
    }
    let config = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or_else(|| usage("no HOME to keep a key in; set LOOPS_KEY_FILE"))?;
    Ok(config.join("loops").join("keys").join("device"))
}
#[cfg(unix)]
fn create_private(path: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}
#[cfg(not(unix))]
fn create_private(path: &Path) -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
}
/// Whether a missing signing key may be made now.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Missing {
    Make,
    Refuse,
}
/// The signing key: `LOOPS_KEY` as seed text, else the key file. The first
/// write on a machine makes the file.
#[expect(
    clippy::print_stderr,
    reason = "Making a key is said once, beside the answer."
)]
fn signing_key(missing: Missing) -> Result<Key, Fail> {
    if let Ok(seed) = env::var("LOOPS_KEY") {
        return Key::from_text(&seed).ok_or_else(|| usage("LOOPS_KEY is not a 32-byte seed"));
    }
    let path = key_file()?;
    match fs::read_to_string(&path) {
        Ok(seed) => Key::from_text(&seed)
            .ok_or_else(|| usage(format!("{} is not a 32-byte seed", path.display()))),
        Err(error) if error.kind() == io::ErrorKind::NotFound && missing == Missing::Make => {
            let key = Key::generate().map_err(|e| Fail::System(e.to_string()))?;
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            let mut file = create_private(&path)?;
            writeln!(file, "{}", key.text())?;
            eprintln!(
                "loops: made signing key {} at {}",
                key.public(),
                path.display()
            );
            Ok(key)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(usage(format!(
            "no signing key at {}; the first emit makes one",
            path.display()
        ))),
        Err(error) => Err(error.into()),
    }
}
/// Nothing yet binds a key to an observer, so the only observer this key can
/// honestly write as is itself. Lineages and key bindings (step 3) widen this.
/// An observer that is not a typed reference at all, left mistyped in a
/// changeset line among them, is the writer's to refuse with its own code,
/// `observer-not-typed`, so it is not judged here.
fn bound<'a>(key: &Key, drafts: impl IntoIterator<Item = &'a batch::Draft>) -> Result<(), Fail> {
    match drafts
        .into_iter()
        .find(|d| envelope::typed_ref(&d.env.observer) && d.env.observer != key.public())
    {
        Some(draft) => Err(refused(format!(
            "observer-unbound: {} is not this key",
            draft.env.observer
        ))),
        None => Ok(()),
    }
}

/// One id for a prefix the person typed.
fn one_id(held: &Held, given: &str) -> Result<String, Fail> {
    match held.ids_from(given).as_slice() {
        [id] => Ok((*id).to_owned()),
        [] => Err(refused(format!("id-unknown: {given}"))),
        ids => Err(refused(format!(
            "id-ambiguous: {given} names {} records",
            ids.len()
        ))),
    }
}
/// A full record id passes through, so a writer can name what it doesn't hold
/// and be refused by the batch check. Anything shorter must name one held record.
fn link(held: &Held, given: &str) -> Result<String, Fail> {
    if loops::envelope::record_id(given) || batch::is_label(given) {
        Ok(given.to_owned())
    } else {
        one_id(held, given)
    }
}
/// A thing: a natural key as typed, or a record id, which may be a prefix of
/// a held one. Text with a colon that isn't `r:` passes through, so the writer
/// can refuse it as `of-not-typed`.
fn thing_ref(held: &Held, given: &str) -> Result<String, Fail> {
    if given.contains(':') && !given.starts_with("r:") {
        Ok(given.to_owned())
    } else {
        link(held, given)
    }
}

fn content_order(a: &Record, b: &Record) -> Ordering {
    if a.newer(b) {
        Ordering::Greater
    } else if b.newer(a) {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}
/// The store as a reader sees it: settled, then admitted. Standing is step 3,
/// so every replacement of a held, related record is permitted.
fn realm(held: &Held) -> Vec<Record> {
    things::admit(&held.settled(), |_, _| true)
}
/// The line to show for a settled record: without its opening once erased.
fn shown_line(held: &Held, record: &Record) -> Option<Json> {
    let line = held.line(&id_text(&record.id))?;
    Some(if record.shown == Shown::Here {
        line.json()
    } else {
        line.erased().json()
    })
}

/// Every write, one record or a changeset, goes through the batch writer,
/// against what this store holds: an unheld `prev` target is `prev-unknown`
/// and an unheld `under` target `under-unknown`. A record whose payload is
/// erased or withheld is held (`Held::writer_view` reads its envelope).
fn write_changeset(
    place: &Dir,
    held: &Held,
    key: &Key,
    drafts: &[batch::Draft],
) -> Result<Vec<Line>, Fail> {
    let lines = prepare_changeset(&|id| held.writer_view(id), key, drafts)?;
    place.append(&lines)?;
    Ok(lines)
}

fn prepare_changeset(
    view: &impl Fn(&str) -> Option<batch::Held>,
    key: &Key,
    drafts: &[batch::Draft],
) -> Result<Vec<Line>, Fail> {
    let written = batch::resolve_batch(&hash, view, drafts).map_err(|e| refused(e.code()))?;
    Ok(written.iter().map(|w| Line::sign(w, key)).collect())
}

fn base_forms(held: &Held, selector: draft::BaseArgs) -> Result<Vec<Record>, Fail> {
    let mut records = Vec::new();
    let mut seen = std::collections::HashSet::new();
    if let Some(given) = selector.tick {
        let id = one_id(held, &given)?;
        let line = held
            .line(&id)
            .ok_or_else(|| refused(format!("id-unknown: {id}")))?;
        if line.env().kind != "tick" {
            return Err(refused("tick-kind at kind"));
        }
        let payload = line
            .payload()
            .filter(|_| tick_open(&held_forms(held), &line.record().id))
            .ok_or_else(|| refused("the tick's payload is not held"))?;
        let members = read_tick_members(&hash, &Rules::current(), payload)
            .map_err(|e| refused(format!("{} at {}", e.code(), e.path)))?;
        for digest in members.covers() {
            let covered = id_text(digest);
            let line = held
                .line(&covered)
                .ok_or_else(|| refused(format!("id-unknown: {covered} covered by {id}")))?;
            seen.insert(covered);
            records.push(line.record());
        }
    }
    for given in selector.ids {
        let id = link(held, &given)?;
        if !seen.contains(&id) {
            let line = held
                .line(&id)
                .ok_or_else(|| refused(format!("id-unknown: {id}")))?;
            seen.insert(id);
            records.push(line.record());
        }
    }
    Ok(records)
}

#[expect(
    clippy::print_stderr,
    reason = "Successful based writes explain their anchor on stderr."
)]
fn base_notices(notices: impl IntoIterator<Item = fill::BaseNotice>) {
    use fill::BaseNotice;
    let mut reported = std::collections::HashSet::new();
    for notice in notices {
        match notice {
            BaseNotice::Named(n) => {
                let noun = if n == 1 { "record" } else { "records" };
                eprintln!("the base did not show {n} {noun} that this emit names");
            }
            BaseNotice::Empty(t) => {
                if reported.insert((false, t.clone())) {
                    eprintln!("the base covers no record of {t}");
                }
            }
            BaseNotice::Unadmitted(t) => {
                if reported.insert((true, t.clone())) {
                    eprintln!("no record of {t} in the base is admitted");
                }
            }
        }
    }
}

fn emit(start: &Path, args: &[String]) -> Result<(), Fail> {
    let place = find_store(start)?;
    let held = place.read()?;
    warn_unclean(&held);
    let key = signing_key(Missing::Make)?;
    let now = clock::now().ok_or_else(|| Fail::System("the clock is outside 1970–9999".into()))?;
    if args.first().is_some_and(|a| a == "-")
        && args
            .get(1)
            .is_none_or(|a| matches!(a.as_str(), "--fill" | "--base" | "--base-id"))
    {
        let (mode, selector) = draft::changeset_args(&args[1..])?;
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let rows = draft::from_jsonl(&input, &key.public(), &now, mode)?;
        bound(&key, rows.iter().map(|s| &s.draft))?;
        let (drafts, notices) = match mode {
            draft::JsonlMode::Plain => (rows.into_iter().map(|s| s.draft).collect(), Vec::new()),
            draft::JsonlMode::Fill => {
                let base = selector.map(|args| base_forms(&held, args)).transpose()?;
                fill::changeset_with_base(&held, &rows, base.as_deref())
                    .map_err(|e| refused(e.code()))?
            }
        };
        let lines = write_changeset(&place.dir, &held, &key, &drafts)?;
        base_notices(notices);
        let records = drafts
            .iter()
            .zip(&lines)
            .map(|(draft, line)| {
                let mut members = vec![("id", text(line.id()))];
                if let Some(label) = &draft.label {
                    members.push(("label", text(label)));
                }
                object(members)
            })
            .collect();
        return say(&object(vec![("records", Json::Arr(records))]).canon());
    }
    write_one(&place.dir, &held, &key, &now, args, None)
}

fn write_one(
    place: &Dir,
    held: &Held,
    key: &Key,
    now: &str,
    args: &[String],
    tick_spec: Option<Spec>,
) -> Result<(), Fail> {
    let (statement, selector) = draft::from_args(args, tick_spec, now, &key.public(), held)?;
    let (draft, notices) = if let Some(args) = selector {
        fill::one_with_base(held, &base_forms(held, args)?, statement)
    } else {
        fill::one(&|id| held.writer_view(id), &realm(held), statement).map(|d| (d, Vec::new()))
    }
    .map_err(|e| refused(e.code()))?;
    bound(key, [&draft])?;
    let lines = write_changeset(place, held, key, &[draft])?;
    base_notices(notices);
    say(lines.first().map_or("", Line::id))
}

fn erase(start: &Path, args: &[String]) -> Result<(), Fail> {
    let place = find_store(start)?;
    let held = place.read()?;
    warn_unclean(&held);
    let key = signing_key(Missing::Make)?;
    let now = clock::now().ok_or_else(|| Fail::System("the clock is outside 1970–9999".into()))?;
    let mut ids = Vec::new();
    let mut why = None;
    for arg in args {
        match held.ids_from(arg).as_slice() {
            [id] => ids.push((*id).to_owned()),
            [] if why.is_none() && !loops::envelope::record_id(arg) => why = Some(arg.clone()),
            [] => return Err(refused(format!("id-unknown: {arg}"))),
            found => {
                return Err(refused(format!(
                    "id-ambiguous: {arg} names {} records",
                    found.len()
                )));
            }
        }
    }
    if ids.is_empty() {
        return Err(usage("erase takes at least one held record id"));
    }
    let draft = batch::Draft {
        label: None,
        env: loops::envelope::Env {
            kind: "erasure".into(),
            observer: key.public(),
            emitted: now,
            erases: ids.clone(),
            ..Default::default()
        },
        salt: loops_place::key::salt().map_err(|e| Fail::System(e.to_string()))?,
        payload: Json::Obj(
            why.map(|w| ("text".to_owned(), text(w)))
                .into_iter()
                .collect(),
        ),
    };
    bound(&key, [&draft])?;
    let lines = write_changeset(&place.dir, &held, &key, &[draft])?;
    let gone = place.dir.purge(&ids)?;
    say(&object(vec![
        ("id", text(lines.first().map_or("", Line::id))),
        ("openings-removed", count(gone)),
    ])
    .canon())
}

fn get(start: &Path, args: &[String]) -> Result<(), Fail> {
    let [given] = args else {
        return Err(usage("get takes one id"));
    };
    let held = find_store(start)?.read()?;
    let id = one_id(&held, given)?;
    let record = held
        .settled()
        .into_iter()
        .find(|r| id_text(&r.id) == id)
        .ok_or_else(|| refused(format!("id-unknown: {given}")))?;
    let line = shown_line(&held, &record).ok_or_else(|| refused(format!("id-unknown: {given}")))?;
    answered(&held, say(&line.canon()))
}

fn of(start: &Path, args: &[String], format: Format) -> Result<(), Fail> {
    let [thing] = args else {
        return Err(usage("of takes one thing"));
    };
    let held = find_store(start)?.read()?;
    let thing = if loops::envelope::typed_ref(thing) && !thing.starts_with("r:") {
        thing.clone()
    } else {
        one_id(&held, thing)?
    };
    // A thing's records are those of it, by `of` or a root's own id; a record
    // that only mentions it in `about` is not one of them. Its values are
    // read from its live setters, and its envelope from its heads, newest
    // first: the first is the winner a latest-by that follows `prev` names.
    let admitted = realm(&held);
    let records: Vec<_> = admitted.iter().filter(|r| r.is_of(&thing)).collect();
    let n = records.len();
    let state = loops::live::LiveState::new(records);
    let heads = state.heads();
    if format == Format::Plain {
        return answered(&held, say(&plain::thing(&held, &thing, n, &state, &heads)?));
    }
    let mut members = vec![
        ("thing", text(&thing)),
        (
            "heads",
            Json::Arr(heads.iter().filter_map(|r| shown_line(&held, r)).collect()),
        ),
        ("records", count(n)),
    ];
    members.extend(values::thing(&held, &state));
    answered(&held, say(&object(members).canon()))
}

fn parse_spec(spec: &str) -> Result<Spec, Fail> {
    let json = loops_eval::parse_json(spec).map_err(|e| refused(format!("spec-not-json: {e}")))?;
    Spec::of_json(&json).map_err(|e| refused(format!("{} at {}", e.code(), e.path)))
}
fn fold_answer(held: &Held, spec: &Spec, view: View, format: Format) -> Result<(), Fail> {
    let admitted = realm(held);
    let result = spec.run_admitted(&admitted);
    if format == Format::Plain {
        return say(&plain::fold(held, &admitted, result, view)?);
    }
    let slots = loops_eval::slots_json(result);
    match view {
        View::Ids => say(&slots.canon()),
        View::Values => say(&values::fold(held, &admitted, spec, slots).canon()),
    }
}

enum SpecInput {
    Vertex,
    File,
}

fn fold(
    start: &Path,
    args: &[String],
    format: Format,
    input: SpecInput,
    overrides: registry::Bindings,
) -> Result<(), Fail> {
    let start = start.canonicalize()?;
    let operands: Vec<_> = args.iter().filter(|arg| *arg != "--values").collect();
    let view = if operands.len() == args.len() {
        View::Ids
    } else {
        View::Values
    };
    let current = current_root(&start);
    let raw = match (input, operands.as_slice()) {
        (SpecInput::File, [file]) => spec_file(&start, file)?,
        (SpecInput::Vertex, [vertex]) => {
            if !vertex_name(vertex) {
                return Err(usage("a vertex name is a file name without its .json"));
            }
            let root = current.ok_or_else(|| no_store(&start))?;
            vertex_text(&root.join(".loops"), vertex)?
        }
        (SpecInput::File, _) => return Err(usage("fold takes one spec file, or - for stdin")),
        (SpecInput::Vertex, _) => return Err(usage("read takes one vertex name")),
    };
    let spec = parse_spec(&raw)?;
    let (held, dirty) = registry::read(&spec, current, overrides)?;
    let answer = fold_answer(&held, &spec, view, format);
    finish_answer(
        answer,
        unclean(&held).map(|why| format!("{why} (stores: {})", dirty.join(", "))),
    )
}
fn spec_file(start: &Path, file: &str) -> Result<String, Fail> {
    if file == "-" {
        let mut spec = String::new();
        io::stdin().read_to_string(&mut spec)?;
        Ok(spec)
    } else {
        Ok(fs::read_to_string(start.join(file))?)
    }
}
fn vertex_name(name: &str) -> bool {
    !name.contains(['/', '\\']) && !name.starts_with('.')
}
fn vertex_text(root: &Path, vertex: &str) -> Result<String, Fail> {
    let path = root.join("vertices").join(format!("{vertex}.json"));
    fs::read_to_string(&path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            refused(format!("vertex-unknown: {}", path.display()))
        } else {
            e.into()
        }
    })
}

fn held_forms(held: &Held) -> Vec<Record> {
    held.lines().iter().map(Line::record).collect()
}

fn check_held_tick(start: &Path, args: &[String]) -> Result<(), Fail> {
    let [given] = args else {
        return Err(usage("tick --check takes one record id or unique prefix"));
    };
    let held = find_store(start)?.read()?;
    let id = one_id(&held, given)?;
    let line = held
        .line(&id)
        .ok_or_else(|| refused(format!("id-unknown: {given}")))?;
    let forms = held_forms(&held);
    let subject = TickSubject {
        kind: &line.env().kind,
        payload: line
            .payload()
            .filter(|_| tick_open(&forms, &line.record().id)),
    };
    let check = check_tick(&hash, &Rules::current(), subject, &forms);
    let same = check.answer == TickAnswer::Same;
    answered(&held, say(&loops_eval::tick_check_json(check).canon()))?;
    if same { Ok(()) } else { Err(Fail::TickNotSame) }
}

fn tick(start: &Path, args: &[String]) -> Result<(), Fail> {
    let Some((source, flags)) = args.split_first() else {
        return Err(usage("tick takes a vertex, a spec file, or - for stdin"));
    };
    if source == "--check" {
        return check_held_tick(start, flags);
    }
    let place = find_store(start)?;
    let input = if source == "-" || !vertex_name(source) || start.join(source).is_file() {
        spec_file(start, source)?
    } else {
        vertex_text(place.dir.root(), source)?
    };
    let spec = parse_spec(&input)?;
    let held = place.read()?;
    warn_unclean(&held);
    let key = signing_key(Missing::Make)?;
    let now = clock::now().ok_or_else(|| Fail::System("the clock is outside 1970–9999".into()))?;
    write_one(&place.dir, &held, &key, &now, flags, Some(spec))
}

fn export(start: &Path, args: &[String]) -> Result<(), Fail> {
    // As a spec's `from` reads them: `--of` takes the records of a thing,
    // `--about` those whose `about` mentions it, as written.
    let mut kind = None;
    let mut thing = None;
    let mut mention = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let slot = match arg.as_str() {
            "--kind" => &mut kind,
            "--of" => &mut thing,
            "--about" => &mut mention,
            other => return Err(usage(format!("export does not take {other}"))),
        };
        *slot = Some(
            rest.next()
                .ok_or_else(|| usage(format!("{arg} takes a value")))?,
        );
    }
    let held = find_store(start)?.read()?;
    let mut records = held.settled();
    records.sort_by(content_order);
    let mut out = io::stdout().lock();
    let written = records
        .iter()
        .filter(|record| {
            kind.is_none_or(|k| *k == record.kind)
                && thing.is_none_or(|t| record.is_of(t))
                && mention.is_none_or(|t| record.about.contains(t))
        })
        .filter_map(|record| shown_line(&held, record))
        .try_for_each(|line| writeln!(out, "{}", line.canon()));
    answered(&held, written.map_err(Fail::from))
}

fn import(start: &Path, args: &[String]) -> Result<(), Fail> {
    let mut input = String::new();
    match args {
        [] => {
            io::stdin().read_to_string(&mut input)?;
        }
        [file] if file == "-" => {
            io::stdin().read_to_string(&mut input)?;
        }
        [file] => input = fs::read_to_string(start.join(file))?,
        _ => return Err(usage("import takes one file, or - for stdin")),
    }
    let place = find_store(start)?;
    let mut lines = Vec::new();
    for (i, line) in input.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        lines.push(
            Line::parse(line).map_err(|e| refused(format!("{} at line {}", e.code(), i + 1)))?,
        );
    }
    let segments = place.dir.append(&lines)?;
    say(&object(vec![
        ("lines", count(lines.len())),
        (
            "segments",
            Json::Arr(segments.into_iter().map(text).collect()),
        ),
    ])
    .canon())
}

fn check(start: &Path) -> Result<(), Fail> {
    let held = find_store(start)?.dir.read()?;
    answered(&held, say(&check_report(&held).canon()))
}
fn check_report(held: &Held) -> Json {
    let refusals = held
        .refused()
        .iter()
        .map(|r| {
            object(vec![
                ("segment", text(&r.segment)),
                ("line", count(r.line)),
                ("refused", text(r.refusal.code())),
            ])
        })
        .collect();
    object(vec![
        ("segments", count(held.segments())),
        ("lines", count(held.lines().len())),
        ("records", count(held.settled().len())),
        ("refused", Json::Arr(refusals)),
        (
            "unbound",
            count(
                held.lines()
                    .iter()
                    .filter(|l| l.env().observer != l.key())
                    .count(),
            ),
        ),
        (
            "altered",
            Json::Arr(held.altered().iter().map(text).collect()),
        ),
    ])
}

fn run(args: &[String]) -> Result<(), Fail> {
    let mut start = PathBuf::from(".");
    let mut rest = Vec::new();
    let mut json = false;
    let mut overrides = registry::Bindings::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-C" if rest.is_empty() => {
                start = args
                    .next()
                    .ok_or_else(|| usage("-C takes a directory"))?
                    .into();
            }
            "--" if rest
                .first()
                .is_some_and(|v: &String| matches!(v.as_str(), "emit" | "tick")) =>
            {
                rest.push(arg.clone());
                rest.extend(args.cloned());
                break;
            }
            "--store" => match rest.first().map(String::as_str) {
                Some("read" | "fold") => {
                    let operand = args
                        .next()
                        .ok_or_else(|| usage("--store takes <name>=<dir>"))?;
                    registry::bind(&mut overrides, &start, operand)?;
                }
                // A preceding option can take this text literally, e.g.
                // `emit --sets --store`. Let the verb's parser decide.
                Some("emit" | "ls" | "export" | "tick")
                    if rest
                        .last()
                        .is_some_and(|v| v.starts_with("--") && v != "--check") =>
                {
                    rest.push(arg.clone());
                }
                _ => return Err(usage("--store is an option of read and fold")),
            },
            "--json" => json = true,
            _ => rest.push(arg.clone()),
        }
    }
    let format = if rest
        .first()
        .is_some_and(|v| matches!(v.as_str(), "ls" | "of" | "read" | "fold"))
        && rest.iter().any(|a| a == "--plain")
    {
        if json {
            return Err(usage("--plain and --json cannot be combined"));
        }
        rest.retain(|a| a != "--plain");
        Format::Plain
    } else {
        Format::Json
    };
    let Some((verb, rest)) = rest.split_first() else {
        return Err(usage(USAGE));
    };
    match verb.as_str() {
        "init" => {
            let place = Dir::init(&start.join(".loops"))?;
            say(&place.root().display().to_string())
        }
        "emit" => emit(&start, rest),
        "tick" => tick(&start, rest),
        "erase" => erase(&start, rest),
        "--version" if rest.is_empty() => {
            say(concat!("strange-loops-next ", env!("CARGO_PKG_VERSION")))
        }
        "ls" => ls::run(&start, rest, format),
        "of" => of(&start, rest, format),
        "get" => get(&start, rest),
        "read" => fold(&start, rest, format, SpecInput::Vertex, overrides),
        "fold" => fold(&start, rest, format, SpecInput::File, overrides),
        "export" => export(&start, rest),
        "import" => import(&start, rest),
        "shadow-import" => shadow::run(&start, rest),
        "check" => check(&start),
        "whoami" if rest == ["--init"] => say(&signing_key(Missing::Make)?.public()),
        "whoami" if rest.iter().any(|arg| arg == "--init") => {
            Err(usage("whoami --init takes no other arguments"))
        }
        "whoami" => {
            let key = signing_key(Missing::Refuse)?;
            say(&object(vec![
                ("key", text(key.public())),
                ("observer", text(key.public())),
            ])
            .canon())
        }
        "help" | "--help" | "-h" => say(USAGE),
        other => Err(usage(format!("unknown verb {other}\n{USAGE}"))),
    }
}

#[expect(
    clippy::print_stderr,
    reason = "The CLI reports refusals and failures on stderr."
)]
fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) | Err(Fail::Closed) => ExitCode::SUCCESS,
        Err(Fail::TickNotSame) => ExitCode::from(1),
        Err(Fail::Refused(code)) => {
            eprintln!("refused: {code}");
            ExitCode::from(1)
        }
        Err(Fail::Usage(message)) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
        Err(Fail::System(message)) => {
            eprintln!("loops: {message}");
            ExitCode::from(3)
        }
    }
}
