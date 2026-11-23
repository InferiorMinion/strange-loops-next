//! Shadow-import orchestration over a frozen group of exports.
mod journal;
mod mapping;
mod plan;
mod report;
mod source;

use crate::{Fail, refused, say, text, usage};
use journal::{Journal, Pin, Seed};
use loops::{base32, json::Json};
use loops_place::{
    clock,
    dir::Dir,
    key::{self, Key},
    line::hash,
};
use mapping::Unnamed;
use report::{Counts, Report};
use std::{
    collections::HashSet,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

fn mismatch(why: &str) -> Fail {
    refused(format!("shadow-state-mismatch: {why}"))
}
fn digest(bytes: &[u8]) -> String {
    base32::base32(&hash(bytes))
}
fn invalid(why: impl std::fmt::Display) -> Fail {
    refused(format!("shadow-source-invalid: {why}"))
}
#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode {
    Write,
    Check,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Point {
    BeforeReserve,
    Reserved,
    Appended,
}
struct Runtime {
    key: fn() -> Result<Key, Fail>,
    seed: fn(&str, String) -> Result<Seed, Fail>,
    at: fn(Point) -> Result<(), Fail>,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            key: || Key::generate().map_err(|e| Fail::System(e.to_string())),
            seed: |_, fingerprint| {
                Ok(Seed {
                    fingerprint,
                    salt: key::salt().map_err(|e| Fail::System(e.to_string()))?,
                    emitted: clock::now()
                        .ok_or_else(|| Fail::System("clock out of range".into()))?,
                })
            },
            at: |_| Ok(()),
        }
    }
}

struct Input {
    name: String,
    facts: PathBuf,
    map: Result<source::Map, String>,
    dest: Dir,
    pin: Pin,
    _guard: File,
}
struct Group {
    state: PathBuf,
    inputs: Vec<Input>,
}
fn relative(parent: &Path, value: &Json) -> Result<PathBuf, Fail> {
    let s = source::string(value).map_err(invalid)?;
    if s.is_empty() || Path::new(s).is_absolute() {
        return Err(invalid("expected a relative path"));
    }
    Ok(parent.join(s))
}
// Resolve existing ancestors, including symlinks, without creating missing directories.
fn resolve_parent(path: &Path) -> Result<PathBuf, Fail> {
    match path.canonicalize() {
        Ok(existing) => Ok(existing),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let name = path
                .file_name()
                .ok_or_else(|| invalid("ambiguous state parent"))?;
            Ok(resolve_parent(
                path.parent()
                    .ok_or_else(|| invalid("missing state parent"))?,
            )?
            .join(name))
        }
        Err(e) => Err(e.into()),
    }
}
impl Group {
    fn read(path: &Path, unnamed: Unnamed) -> Result<Self, Fail> {
        let bytes = fs::read(path)?;
        let json = loops_eval::parse_json(std::str::from_utf8(&bytes).map_err(invalid)?)
            .map_err(invalid)?;
        let group = source::fields(&json, &["state", "sources"]).map_err(invalid)?;
        let parent = path.parent().unwrap_or(Path::new("."));
        let state = relative(parent, group["state"])?;
        let state_name = state
            .file_name()
            .ok_or_else(|| invalid("state must name a private directory"))?;
        let state = resolve_parent(
            state
                .parent()
                .ok_or_else(|| invalid("missing state parent"))?,
        )?
        .join(state_name);
        let Json::Arr(entries) = group["sources"] else {
            return Err(invalid("expected sources array"));
        };
        if entries.is_empty() {
            return Err(invalid("group has no sources"));
        }
        let mut inputs = Vec::new();
        let mut names = HashSet::new();
        let mut destinations = HashSet::new();
        for entry in entries {
            let e =
                source::fields(entry, &["name", "facts", "map", "destination"]).map_err(invalid)?;
            let name = source::string(e["name"]).map_err(invalid)?.to_owned();
            let map_bytes = fs::read(relative(parent, e["map"])?)?;
            let map = std::str::from_utf8(&map_bytes)
                .map_err(|_| "shadow-map-invalid: expected UTF-8".to_owned())
                .and_then(source::Map::read)
                .and_then(|map| {
                    if name == map.source {
                        Ok(map)
                    } else {
                        Err(format!(
                            "shadow-map-invalid: {name:?} disagrees with map for {:?}",
                            map.store
                        ))
                    }
                });
            let path = relative(parent, e["destination"])?.join(".loops");
            let destination_error = |err: io::Error| {
                if err.kind() == io::ErrorKind::NotFound {
                    mismatch(&format!("{name}: destination has no .loops/segments"))
                } else {
                    err.into()
                }
            };
            let root = path.canonicalize().map_err(destination_error)?;
            let dest = Dir::open(&root).map_err(destination_error)?;
            let segments = root.join("segments").canonicalize()?;
            if state.starts_with(&segments)
                || segments.starts_with(&state)
                || state.starts_with(&root)
                || root.starts_with(&state)
                || !names.insert(name.clone())
                || !destinations.insert(segments.clone())
            {
                return Err(invalid(
                    "duplicate source/destination or private state overlaps a store",
                ));
            }
            let guard = File::open(&segments)?;
            journal::exclusive(&guard)?;
            let pin = Pin {
                destination: root
                    .to_str()
                    .ok_or_else(|| invalid("destination path is not UTF-8"))?
                    .into(),
                map: digest(&map_bytes),
                unnamed: unnamed == Unnamed::Roots,
            };
            inputs.push(Input {
                name,
                facts: relative(parent, e["facts"])?,
                map,
                dest,
                pin,
                _guard: guard,
            });
        }
        Ok(Self { state, inputs })
    }
}
fn event(op: &str, args: impl IntoIterator<Item = Json>) -> Json {
    Json::Arr(std::iter::once(text(op)).chain(args).collect())
}

pub(super) fn run(start: &Path, args: &[String]) -> Result<(), Fail> {
    let (file, flags) = args
        .split_first()
        .ok_or_else(|| usage("shadow-import requires a group file"))?;
    if file.starts_with("--") {
        return Err(usage("shadow-import requires a group file first"));
    }
    let mut unnamed = Unnamed::Omit;
    let mut mode = Mode::Write;
    for flag in flags {
        match flag.as_str() {
            "--include-unnamed" if unnamed == Unnamed::Omit => unnamed = Unnamed::Roots,
            "--check-only" if mode == Mode::Write => mode = Mode::Check,
            _ => return Err(usage(format!("unexpected shadow-import option {flag}"))),
        }
    }
    execute(&start.join(file), unnamed, mode, &Runtime::default())
}

fn execute(path: &Path, unnamed: Unnamed, mode: Mode, runtime: &Runtime) -> Result<(), Fail> {
    let mut report = Report::default();
    let result = import(path, unnamed, mode, runtime, &mut report);
    // Writers report the work actually completed even if a later append failed.
    say(&report.json().canon())?;
    result
}
fn import(
    path: &Path,
    unnamed: Unnamed,
    mode: Mode,
    runtime: &Runtime,
    report: &mut Report,
) -> Result<(), Fail> {
    let group = Group::read(path, unnamed)?;
    let dir = &group.state;
    let exists = journal::state_exists(dir)?;
    let mut lock = if exists {
        journal::lock(dir, Mode::Check)?
    } else {
        None
    };
    let log = dir.join("journal.jsonl");
    let mut j = Journal::read(&log)?;
    let saved_key = journal::key(dir)?;
    let key = match saved_key {
        Some(k) => k,
        None if j.public.is_some() => return Err(mismatch("missing pinned signing key")),
        None => (runtime.key)()?,
    };
    if j.public.as_ref().is_some_and(|p| p != &key.public()) {
        return Err(mismatch("import key changed"));
    }
    let mut new_pins = Vec::new();
    let mut plans = Vec::new();
    let mut identities = plan::Identities::from(&j);
    for input in &group.inputs {
        report.sources.push((input.name.clone(), Counts::default()));
    }
    let (index, cuts) = plan::Index::collect(&group.inputs, unnamed, &j);
    for (i, (input, cut)) in group.inputs.iter().zip(cuts).enumerate() {
        if let Some(pin) = j.pins.get(&input.name) {
            if pin != &input.pin {
                return Err(mismatch(&format!(
                    "{}: map, inclusion flag or destination changed",
                    input.name
                )));
            }
        } else {
            if j.pins
                .values()
                .any(|p| p.destination == input.pin.destination)
            {
                return Err(mismatch("destination already belongs to another source"));
            }
            new_pins.push(input.pin.json(&input.name));
        }
        plans.push(plan::preflight(
            input,
            i,
            &index,
            cut,
            &key,
            &j,
            &mut identities,
            runtime,
            report,
        )?);
    }
    drop(index);
    if !report.errors.is_empty() {
        return Err(refused(report.errors.join("\nrefused: ")));
    }
    if mode == Mode::Check {
        return Ok(());
    }
    if lock.is_none() {
        journal::ensure_dir(dir)?;
        lock = journal::lock(dir, Mode::Write)?;
        let current = Journal::read(&log)?;
        if current.version() != j.version()
            || journal::key(dir)?.is_some_and(|k| k.public() != key.public())
        {
            return Err(mismatch("another writer initialized this group; retry"));
        }
    }
    if journal::key(dir)?.is_none() {
        journal::save_key(dir, &key)?;
    }
    let mut file = j.writer(&log)?;
    if j.public.is_none() {
        j.append(
            &mut file,
            &event("v1", [text(key.public()), Json::Arr(new_pins)]),
        )?;
    } else if !new_pins.is_empty() {
        j.append(&mut file, &event("sources", [Json::Arr(new_pins)]))?;
    }
    for ((input, plan), (_, counts)) in group.inputs.iter().zip(&mut plans).zip(&mut report.sources)
    {
        plan.publish(
            input,
            &key,
            &mut j,
            &mut file,
            &identities.seeds,
            runtime,
            counts,
        )?;
    }
    drop(lock);
    Ok(())
}

#[cfg(test)]
mod tests;
