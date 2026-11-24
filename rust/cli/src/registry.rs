//! Resolve names from a single origin before reading any selected segments.
use crate::{Fail, Store, current_root, refused, usage};
use loops::{json::Json, spec::Spec};
use loops_place::dir::{Dir, Held};
use std::{
    collections::{BTreeMap, HashMap, HashSet, btree_map::Entry},
    env, fs, io,
    path::{Path, PathBuf},
};

pub(crate) struct Binding {
    directory: PathBuf,
    source: String,
}

pub(crate) type Bindings = HashMap<String, Binding>;

pub(crate) fn bind(bindings: &mut Bindings, operand: &str) -> Result<(), Fail> {
    let (name, dir) = operand
        .split_once('=')
        .filter(|(name, dir)| !name.is_empty() && !dir.is_empty())
        .ok_or_else(|| usage("--store takes <name>=<dir>"))?;
    bindings.insert(
        name.to_owned(),
        Binding {
            directory: PathBuf::from(dir),
            source: format!("--store {operand}"),
        },
    );
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
        bindings.insert(
            name,
            Binding {
                directory: dir.join(path),
                source: file.display().to_string(),
            },
        );
    }
    Ok(Some(bindings))
}

/// Keep the first local list whole, including the distinction between absent and empty.
pub(crate) struct Registry {
    user: Bindings,
    pub(crate) local: Option<Bindings>,
    overrides: Bindings,
    current: Option<Binding>,
}

impl Registry {
    pub(crate) fn load(origin: &Path, overrides: Bindings) -> Result<Self, Fail> {
        let config = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
        Self::from_lists(origin, config.as_deref(), overrides)
    }

    fn from_lists(origin: &Path, config: Option<&Path>, overrides: Bindings) -> Result<Self, Fail> {
        let start = origin.canonicalize()?;
        let root = current_root(&start);
        let user = match config {
            Some(config) => list(&config.join("loops"))?.unwrap_or_default(),
            None => Bindings::new(),
        };
        let mut local = list(root.unwrap_or(&start))?;
        if local.is_none()
            && let Some(parent) = root.and_then(Path::parent)
        {
            local = list(parent)?;
        }
        Ok(Self {
            user,
            local,
            overrides,
            current: root.map(|dir| Binding {
                directory: dir.to_owned(),
                source: "current store".to_owned(),
            }),
        })
    }

    fn explicit(&self, name: &str) -> Option<&Binding> {
        self.overrides
            .get(name)
            .or_else(|| self.local.as_ref().and_then(|local| local.get(name)))
            .or_else(|| self.user.get(name))
    }

    fn binding(&self, name: &str) -> Result<&Binding, Fail> {
        self.explicit(name)
            .or_else(|| self.current.as_ref().filter(|_| name == "."))
            .ok_or_else(|| refused(format!("store-unknown: {name}")))
    }

    /// Open exactly this directory's store. Only the unnamed -C lookup walks up.
    pub(crate) fn open(&self, name: &str) -> Result<(Store, PathBuf, &str), Fail> {
        let binding = self.binding(name)?;
        let directory = binding
            .directory
            .canonicalize()
            .map_err(|e| binding.error(name, &e))?;
        let root = directory
            .join(".loops")
            .canonicalize()
            .map_err(|e| binding.error(name, &e))?;
        Ok((binding.open(name, &root)?, directory, &binding.source))
    }

    /// Open all the distinct stores before reading any. Aliases accompany diagnostics.
    pub(crate) fn stores<'a>(
        &self,
        names: impl Iterator<Item = &'a str>,
        working: Option<&Store>,
    ) -> Result<BTreeMap<PathBuf, (Store, Vec<&'a str>)>, Fail> {
        let mut seen = HashSet::new();
        let mut stores = BTreeMap::new();
        for name in names {
            if !seen.insert(name) {
                continue;
            }
            // Only an unbound dot means the working store. It can be outside
            // the origin, even when the origin has no store of its own.
            if name == "."
                && let Some(store) = working
                && self.explicit(".").is_none()
            {
                stores
                    .entry(store.dir.root().to_owned())
                    .or_insert_with(|| (store.clone(), Vec::new()))
                    .1
                    .push(name);
                continue;
            }
            let binding = self.binding(name)?;
            let root = binding.root(name)?;
            match stores.entry(root) {
                Entry::Vacant(entry) => {
                    let store = binding.open(name, entry.key())?;
                    entry.insert((store, vec![name]));
                }
                Entry::Occupied(mut entry) => entry.get_mut().1.push(name),
            }
        }
        Ok(stores)
    }

    pub(crate) fn read<'a>(
        &self,
        spec: &'a Spec,
        working: Option<&Store>,
    ) -> Result<(Held, Vec<&'a str>), Fail> {
        let mut union = Held::default();
        let mut dirty = Vec::new();
        for (store, aliases) in self
            .stores(spec.sources.iter().map(|s| s.store.as_str()), working)?
            .into_values()
        {
            let held = store.read()?;
            if !held.refused().is_empty() || !held.altered().is_empty() {
                dirty.extend(aliases);
            }
            union.append(held);
        }
        dirty.sort_unstable();
        Ok((union, dirty))
    }
}

pub(crate) enum Selection {
    Names(Vec<String>),
    Group,
}

pub(crate) struct SelectedStore {
    pub(crate) directory: PathBuf,
    pub(crate) store: Store,
}

/// Resolve the whole selection before any caller reads a description or segment.
pub(crate) fn selected(
    origin: &Path,
    overrides: Bindings,
    selection: &Selection,
) -> Result<Vec<SelectedStore>, Fail> {
    let registry;
    let names: Vec<_> = match selection {
        Selection::Names(names) if names.is_empty() => {
            let store = crate::find_store(origin)?;
            let directory = store
                .dir
                .root()
                .parent()
                .expect("opened .loops directory")
                .to_owned();
            return Ok(vec![SelectedStore { directory, store }]);
        }
        Selection::Names(names) => {
            registry = Registry::load(origin, overrides)?;
            names.iter().map(String::as_str).collect()
        }
        Selection::Group => {
            // Every group member has a local or command binding.
            registry = Registry::from_lists(origin, None, overrides)?;
            let local = registry
                .local
                .as_ref()
                .ok_or_else(|| usage("--group needs a local stores.json"))?;
            let mut members: Vec<_> = local.keys().map(String::as_str).collect();
            members.sort_by(|a, b| loops::json::str_cmp(a, b));
            members
        }
    };
    registry
        .stores(names.into_iter(), None)?
        .into_values()
        .map(|(store, aliases)| {
            // Store directories may themselves be symlinks, or two distinct
            // directories may link .loops to the same data. Report one real
            // selected directory, independently of the order of the selectors.
            let mut directories = Vec::new();
            for name in aliases {
                let binding = registry.binding(name)?;
                directories.push(
                    binding
                        .directory
                        .canonicalize()
                        .map_err(|e| binding.error(name, &e))?,
                );
            }
            let directory = directories.into_iter().min().expect("one or more aliases");
            Ok(SelectedStore { directory, store })
        })
        .collect()
}

impl Binding {
    pub(crate) fn resolve_directory(&mut self, origin: &Path) {
        self.directory = origin.join(&self.directory);
    }

    fn error(&self, name: &str, e: &io::Error) -> Fail {
        match e.kind() {
            io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => refused(format!(
                "store-unknown: {name}: {}",
                self.directory.display()
            )),
            _ => Fail::System(format!("{}: {e}", self.directory.display())),
        }
    }

    fn root(&self, name: &str) -> Result<PathBuf, Fail> {
        self.directory
            .join(".loops")
            .canonicalize()
            .map_err(|e| self.error(name, &e))
    }

    fn open(&self, name: &str, root: &Path) -> Result<Store, Fail> {
        Ok(Store {
            dir: Dir::open(root).map_err(|e| self.error(name, &e))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_membership_and_opening_multiple_names() {
        let group = env::temp_dir().join(format!("kiln-registry-{}", std::process::id()));
        let yard = group.join("yard");
        Dir::init(&yard.join(".loops")).expect("initialize yard");
        let config = group.join("config");
        fs::create_dir_all(config.join("loops")).expect("user registry directory");
        fs::write(
            config.join("loops/stores.json"),
            r#"{"potter":"../../yard"}"#,
        )
        .expect("user list");
        let resolve = || {
            let mut overrides = Bindings::new();
            bind(&mut overrides, &format!("backups={}", yard.display()))
                .ok()
                .expect("override");
            Registry::from_lists(&yard, Some(&config), overrides)
                .ok()
                .expect("resolve bindings")
        };
        assert!(resolve().local.is_none());
        fs::write(
            group.join("stores.json"),
            r#"{"yard":"yard","workshop":"yard"}"#,
        )
        .expect("group list");
        let registry = resolve();
        let local = registry.local.as_ref().expect("parent list");
        assert_eq!(local.len(), 2);
        assert!(local.contains_key("yard") && local.contains_key("workshop"));
        let names = ["yard", "workshop", "potter", "backups", "yard"];
        let stores = registry
            .stores(names.into_iter(), None)
            .ok()
            .expect("open aliases");
        assert_eq!(stores.len(), 1);
        let (store, aliases) = stores.into_values().next().expect("yard");
        assert_eq!(
            store.dir.root(),
            yard.join(".loops").canonicalize().expect("yard root")
        );
        assert_eq!(aliases, names[..4]);
        assert!(
            matches!(registry.stores(["yard", "kiln"].into_iter(), None),
            Err(Fail::Refused(s)) if s == "store-unknown: kiln")
        );
        fs::write(yard.join("stores.json"), "{}").expect("empty nearer list");
        let registry = resolve();
        let local = registry.local.as_ref().expect("empty local file");
        assert!(local.is_empty());
        assert!(
            registry
                .stores(local.keys().map(String::as_str), None)
                .ok()
                .expect("open empty group")
                .is_empty()
        );
        assert!(registry.open("workshop").is_err());
        fs::remove_dir_all(group).expect("remove fixture");
    }
}
