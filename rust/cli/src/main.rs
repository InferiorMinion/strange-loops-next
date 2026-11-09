//! `loops`: emit, read and fold for one person, over a `.loops/` directory.
//!
//! The verbs follow the design's CLI. Exit codes: 0 ok, 1 refused (with its
//! code), 2 a usage error, 3 the file system or the system's randomness failed.
mod draft;

use loops::{
    batch,
    json::Json,
    record::{Record, Shown, id_text},
    spec::Spec,
    things,
};
use loops_place::{
    clock,
    dir::{Dir, Held},
    key::Key,
    line::{Line, hash},
};
use std::{
    cmp::Ordering,
    env, fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const USAGE: &str = "\
usage: loops [-C <dir>] <verb> ...

  init                          make .loops/ here
  emit <kind> [field=text | field:=json]... [\"message\"]
       [--about <thing>]... [--prev <id>]... [--erases <id>]...
       [--occurred <when>] [--under <id>]
  emit -                        drafts as JSONL on stdin: one changeset
  get <id>                      one record
  about <thing>                 a thing's current state
  read <vertex>                 fold .loops/vertices/<vertex>.json
  fold <spec.json | ->          fold a spec given as a file
  erase <id>... [\"why\"]         emit an erasure, and drop the openings held here
  export [--kind <k>] [--about <thing>]   stored lines, oldest first
  import [<file> | -]           add stored lines; a store is the union
  check                         verify every segment and line
  whoami                        the key and observer this machine writes as";

/// How a command ends without its answer.
enum Fail {
    Usage(String),
    Refused(String),
    System(String),
}
impl From<io::Error> for Fail {
    fn from(error: io::Error) -> Self {
        Self::System(error.to_string())
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

/// Finds the nearest `.loops/` at or above `start`, as git finds `.git`.
fn find_store(start: &Path) -> Result<Dir, Fail> {
    let start = fs::canonicalize(start)?;
    let mut at = Some(start.as_path());
    while let Some(dir) = at {
        if let Ok(place) = Dir::open(&dir.join(".loops")) {
            return Ok(place);
        }
        at = dir.parent();
    }
    Err(usage(format!(
        "no .loops/ at or above {}; run `loops init`",
        start.display()
    )))
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
/// Whether a missing device key may be made now.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Missing {
    Make,
    Refuse,
}
/// The device key: `LOOPS_KEY` as seed text, else the key file. The first
/// write on a machine makes the file.
#[expect(
    clippy::print_stderr,
    reason = "Making a key is said once, beside the answer."
)]
fn device_key(missing: Missing) -> Result<Key, Fail> {
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
                "loops: made device key {} at {}",
                key.public(),
                path.display()
            );
            Ok(key)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(usage(format!(
            "no device key at {}; the first emit makes one",
            path.display()
        ))),
        Err(error) => Err(error.into()),
    }
}
/// Nothing yet binds a key to an observer, so the only observer this key can
/// honestly write as is itself. Lineages and key bindings (step 3) widen this.
fn bound(key: &Key, drafts: &[batch::Draft]) -> Result<(), Fail> {
    match drafts.iter().find(|d| d.env.observer != key.public()) {
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

fn write_changeset(
    place: &Dir,
    held: &Held,
    key: &Key,
    drafts: &[batch::Draft],
) -> Result<Vec<Line>, Fail> {
    bound(key, drafts)?;
    let about_of = |id: &str| held.line(id).map(|line| line.env().about.clone());
    let written = batch::resolve_batch(&hash, &about_of, drafts).map_err(|e| refused(e.code()))?;
    let lines: Vec<Line> = written.iter().map(|w| Line::sign(w, key)).collect();
    place.append(&lines)?;
    Ok(lines)
}

fn emit(start: &Path, args: &[String]) -> Result<(), Fail> {
    let place = find_store(start)?;
    let held = place.read()?;
    let key = device_key(Missing::Make)?;
    let now = clock::now().ok_or_else(|| Fail::System("the clock is outside 1970–9999".into()))?;
    if args == ["-"] {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let drafts = draft::from_jsonl(&input, &key.public(), &now)?;
        let lines = write_changeset(&place, &held, &key, &drafts)?;
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
    let draft = draft::from_args(args, &now, &key.public(), |given| link(&held, given))?;
    let lines = write_changeset(&place, &held, &key, &[draft])?;
    say(lines.first().map_or("", Line::id))
}

fn erase(start: &Path, args: &[String]) -> Result<(), Fail> {
    let place = find_store(start)?;
    let held = place.read()?;
    let key = device_key(Missing::Make)?;
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
    let lines = write_changeset(&place, &held, &key, &[draft])?;
    let gone = place.purge(&ids)?;
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
    say(&line.canon())
}

fn about(start: &Path, args: &[String]) -> Result<(), Fail> {
    let [thing] = args else {
        return Err(usage("about takes one thing"));
    };
    let held = find_store(start)?.read()?;
    let thing = if loops::envelope::typed_ref(thing) && !thing.starts_with("r:") {
        thing.clone()
    } else {
        one_id(&held, thing)?
    };
    let admitted = realm(&held);
    let state = things::state_in(&thing, &admitted);
    let lines = |records: &[Record]| {
        Json::Arr(
            records
                .iter()
                .filter_map(|r| shown_line(&held, r))
                .collect(),
        )
    };
    say(&object(vec![
        ("thing", text(&thing)),
        (
            "winner",
            state
                .winner()
                .and_then(|r| shown_line(&held, r))
                .unwrap_or(Json::Null),
        ),
        ("others", lines(state.others())),
        (
            "records",
            count(admitted.iter().filter(|r| r.is_about(&thing)).count()),
        ),
    ])
    .canon())
}

fn fold_text(start: &Path, spec: &str) -> Result<(), Fail> {
    let json = loops_eval::parse_json(spec).map_err(|e| refused(format!("spec-not-json: {e}")))?;
    let spec = Spec::of_json(&json).map_err(|e| refused(format!("{} at {}", e.code(), e.path)))?;
    let held = find_store(start)?.read()?;
    say(&loops_eval::slots_json(spec.run(&held.settled())).canon())
}
fn fold(start: &Path, args: &[String]) -> Result<(), Fail> {
    let [file] = args else {
        return Err(usage("fold takes one spec file, or - for stdin"));
    };
    let mut spec = String::new();
    if file == "-" {
        io::stdin().read_to_string(&mut spec)?;
    } else {
        spec = fs::read_to_string(start.join(file))?;
    }
    fold_text(start, &spec)
}
fn read(start: &Path, args: &[String]) -> Result<(), Fail> {
    let [vertex] = args else {
        return Err(usage("read takes one vertex name"));
    };
    if vertex.contains(['/', '\\']) || vertex.starts_with('.') {
        return Err(usage("a vertex name is a file name without its .json"));
    }
    let place = find_store(start)?;
    let path = place.root().join("vertices").join(format!("{vertex}.json"));
    let spec = fs::read_to_string(&path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            refused(format!("vertex-unknown: {}", path.display()))
        } else {
            e.into()
        }
    })?;
    fold_text(start, &spec)
}

fn export(start: &Path, args: &[String]) -> Result<(), Fail> {
    let mut kind = None;
    let mut thing = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let slot = match arg.as_str() {
            "--kind" => &mut kind,
            "--about" => &mut thing,
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
    for record in records {
        if kind.is_some_and(|k| *k != record.kind) || thing.is_some_and(|t| !record.is_about(t)) {
            continue;
        }
        if let Some(line) = shown_line(&held, &record) {
            writeln!(out, "{}", line.canon())?;
        }
    }
    Ok(())
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
    let segments = place.append(&lines)?;
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
    let held = find_store(start)?.read()?;
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
    say(&object(vec![
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
    .canon())?;
    if held.refused().is_empty() && held.altered().is_empty() {
        Ok(())
    } else {
        Err(refused("store-not-clean"))
    }
}

fn run(args: &[String]) -> Result<(), Fail> {
    let mut start = PathBuf::from(".");
    let mut rest = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-C" if rest.is_empty() => {
                start = args
                    .next()
                    .ok_or_else(|| usage("-C takes a directory"))?
                    .into();
            }
            // Every answer is already JSON or an id; a lens arrives with a later step.
            "--json" => {}
            _ => rest.push(arg.clone()),
        }
    }
    let Some((verb, rest)) = rest.split_first() else {
        return Err(usage(USAGE));
    };
    match verb.as_str() {
        "init" => {
            let place = Dir::init(&start.join(".loops"))?;
            say(&place.root().display().to_string())
        }
        "emit" => emit(&start, rest),
        "erase" => erase(&start, rest),
        "get" => get(&start, rest),
        "about" => about(&start, rest),
        "read" => read(&start, rest),
        "fold" => fold(&start, rest),
        "export" => export(&start, rest),
        "import" => import(&start, rest),
        "check" => check(&start),
        "whoami" => {
            let key = device_key(Missing::Refuse)?;
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
        Ok(()) => ExitCode::SUCCESS,
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
