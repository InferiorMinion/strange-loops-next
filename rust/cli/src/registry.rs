//! Resolve the stores named by a spec before reading any of their segments.
use crate::{Fail, Store, refused, usage};
use loops::{json::Json, spec::Spec};
use loops_place::dir::{Dir, Held};
use std::{
    collections::{BTreeMap, HashMap, HashSet, btree_map::Entry},
    env, fs, io,
    path::{Path, PathBuf},
};

pub(crate) type Bindings = HashMap<String, PathBuf>;

pub(crate) fn bind(bindings: &mut Bindings, start: &Path, operand: &str) -> Result<(), Fail> {
    let (name, dir) = operand
        .split_once('=')
        .filter(|(name, dir)| !name.is_empty() && !dir.is_empty())
        .ok_or_else(|| usage("--store takes <name>=<dir>"))?;
    bindings.insert(name.to_owned(), start.join(dir));
    Ok(())
}

fn list(dir: &Path) -> Result<Option<Bindings>, Fail> {
    let file = dir.join("stores.json");
    let invalid = |why: &str| usage(format!("{}: {why}", file.display()));
    let input = match fs::read_to_string(&file) {
        Ok(input) => input,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) if e.kind() == io::ErrorKind::InvalidData => return Err(invalid(&e.to_string())),
        Err(e) => return Err(Fail::System(format!("{}: {e}", file.display()))),
    };
    let Json::Obj(members) = loops_eval::parse_json(&input).map_err(|e| invalid(&e.to_string()))?
    else {
        return Err(invalid("expected an object of store names and directories"));
    };
    let mut bindings = Bindings::new();
    for (name, value) in members {
        let Json::Str(path) = value else {
            return Err(invalid(&format!("expected a directory string for {name}")));
        };
        if bindings.contains_key(&name) {
            return Err(invalid(&format!("duplicate store name {name}")));
        }
        bindings.insert(name, dir.join(path));
    }
    Ok(Some(bindings))
}

fn bindings(start: &Path, current: Option<&Path>, overrides: Bindings) -> Result<Bindings, Fail> {
    let config = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    let mut names = Bindings::new();
    if let Some(config) = config {
        names = list(&config.join("loops"))?.unwrap_or_default();
    }
    if let Some(root) = current {
        names
            .entry(".".to_owned())
            .or_insert_with(|| root.to_owned());
        for dir in root.ancestors().take(2) {
            if let Some(local) = list(dir)? {
                names.extend(local);
                break;
            }
        }
    } else if let Some(local) = list(start)? {
        names.extend(local);
    }
    names.extend(overrides);
    Ok(names)
}

fn stores<'a>(
    spec: &'a Spec,
    names: &Bindings,
) -> Result<BTreeMap<PathBuf, (Store, Vec<&'a str>)>, Fail> {
    let mut seen = HashSet::new();
    let mut stores = BTreeMap::new();
    for name in spec.sources.iter().map(|s| s.store.as_str()) {
        if !seen.insert(name) {
            continue;
        }
        let path = names
            .get(name)
            .ok_or_else(|| refused(format!("store-unknown: {name}")))?;
        let open_error = |e: io::Error| match e.kind() {
            io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => {
                refused(format!("store-unknown: {name}: {}", path.display()))
            }
            _ => Fail::System(format!("{}: {e}", path.display())),
        };
        let root = path.join(".loops").canonicalize().map_err(open_error)?;
        match stores.entry(root) {
            Entry::Vacant(entry) => {
                let dir = Dir::open(entry.key()).map_err(open_error)?;
                entry.insert((Store { dir }, vec![name]));
            }
            Entry::Occupied(mut entry) => entry.get_mut().1.push(name),
        }
    }
    Ok(stores)
}

/// Names of unclean stores accompany the union so the caller can answer first.
pub(crate) fn read<'a>(
    spec: &'a Spec,
    start: &Path,
    current: Option<&Path>,
    overrides: Bindings,
) -> Result<(Held, Vec<&'a str>), Fail> {
    let names = bindings(start, current, overrides)?;
    let mut union = Held::default();
    let mut dirty = Vec::new();
    for (store, aliases) in stores(spec, &names)?.into_values() {
        let held = store.read()?;
        if !held.refused().is_empty() || !held.altered().is_empty() {
            dirty.extend(aliases);
        }
        union.append(held);
    }
    dirty.sort_unstable();
    Ok((union, dirty))
}
