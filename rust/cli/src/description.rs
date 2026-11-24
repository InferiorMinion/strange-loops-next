//! The optional, store-local description used by the CLI's listings and plain views.
use crate::{Fail, plain::label, usage};
use loops::{envelope::typed_ref, json::Json, record::Record, where_::Field};
use std::{
    collections::{BTreeMap, HashSet},
    fs, io,
    path::Path,
};

#[derive(Default)]
pub(crate) struct Description {
    note: Option<String>,
    kinds: BTreeMap<String, Kind>,
}

#[derive(Default)]
struct Kind {
    note: Option<String>,
    key: Key,
    key_format: Option<String>,
    headline: Option<String>,
    refs: Option<BTreeMap<String, Vec<String>>>,
    lifecycle: Option<Lifecycle>,
    preview: Option<Vec<String>>,
    search: Option<Vec<String>>,
}

#[derive(Default)]
enum Key {
    #[default]
    Omitted,
    Null,
    Field(String),
}

pub(crate) struct Lifecycle {
    pub(crate) field: String,
    active: Option<Vec<String>>,
    closed: Option<Vec<String>>,
}

impl Description {
    /// The directory is the working store's opened .loops directory.
    pub(crate) fn load(dir: &Path) -> Result<Option<Self>, Fail> {
        let path = dir.join("store.json");
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(Fail::System(format!("{}: {e}", path.display()))),
        };
        let invalid = |why: String| usage(format!("{}: {why}", path.display()));
        let input = std::str::from_utf8(&bytes).map_err(|e| invalid(format!("$: {e}")))?;
        let json = loops_eval::parse_json(input).map_err(|e| invalid(format!("$: {e}")))?;
        Self::parse(json).map(Some).map_err(invalid)
    }

    fn parse(json: Json) -> Result<Self, String> {
        let mut desc = Self::default();
        let mut version = false;
        let mut kinds = false;
        members(json, "$", |key, at, val| {
            match key.as_str() {
                "version" if val == Json::Int(1_u64.into()) => version = true,
                "version" => return Err(format!("{at}: expected version 1")),
                "note" => desc.note = Some(string(val, at)?),
                "kinds" => {
                    members(val, at, |name, at, val| {
                        if name.is_empty() {
                            return Err(format!("{at}: expected a nonempty kind name"));
                        }
                        let kind = Kind::parse(val, at, &name)?;
                        desc.kinds.insert(name, kind);
                        Ok(())
                    })?;
                    kinds = true;
                }
                _ => return Err(format!("{at}: unknown member")),
            }
            Ok(())
        })?;
        if !version {
            return Err("version: missing member".into());
        }
        if !kinds {
            return Err("kinds: missing member".into());
        }
        Ok(desc)
    }

    /// A field configured by the heads. Disagreement uses the historical name
    /// field. Without any configuration, each view keeps its original layout.
    pub(crate) fn headline<'a>(&'a self, heads: &[Record]) -> Option<&'a str> {
        let mut common = None;
        let mut configured = false;
        for head in heads {
            let field = self.kinds.get(&head.kind).and_then(Kind::headline);
            configured |= field.is_some();
            let effective = field.unwrap_or("name");
            if common.is_some_and(|previous| previous != effective) {
                return Some("name");
            }
            common = Some(effective);
        }
        common.filter(|_| configured)
    }

    pub(crate) fn preview(&self, prefix: &str) -> Option<&[String]> {
        self.kinds.get(prefix_kind(prefix)?)?.preview.as_deref()
    }

    pub(crate) fn lifecycle(&self, kind: &str) -> Option<&Lifecycle> {
        self.kinds.get(kind)?.lifecycle.as_ref()
    }
}

pub(crate) fn prefix_kind(prefix: &str) -> Option<&str> {
    let (kind, _) = prefix.split_once(':')?;
    natural_kind(kind).then_some(kind)
}

// The reserved record-ID prefix selects roots across kinds.
fn natural_kind(name: &str) -> bool {
    name != "r" && !name.contains(':') && typed_ref(&format!("{name}:x"))
}

impl Kind {
    fn headline(&self) -> Option<&str> {
        self.headline.as_deref().or(match &self.key {
            Key::Field(name) => Some(name.as_str()),
            Key::Null | Key::Omitted => None,
        })
    }

    fn parse(json: Json, path: &str, name: &str) -> Result<Self, String> {
        let mut kind = Self::default();
        members(json, path, |key, at, val| {
            match key.as_str() {
                "note" => kind.note = Some(string(val, at)?),
                "key_format" => kind.key_format = Some(string(val, at)?),
                "headline" => kind.headline = Some(nonempty(val, at)?),
                "key" if val == Json::Null => kind.key = Key::Null,
                "key" => {
                    let field = nonempty(val, at)?;
                    if !natural_kind(name) {
                        return Err(format!(
                            "{at}: keyed kind needs a natural prefix other than r"
                        ));
                    }
                    kind.key = Key::Field(field);
                }
                "refs" => {
                    let mut refs = BTreeMap::new();
                    members(val, at, |field, at, targets| {
                        if !matches!(Field::of_name(&field), Some(Field::Payload(_))) {
                            return Err(format!("{at}: expected a payload field name"));
                        }
                        refs.insert(field, list(targets, at, nonempty)?);
                        Ok(())
                    })?;
                    kind.refs = Some(refs);
                }
                "lifecycle" => kind.lifecycle = Some(Lifecycle::parse(val, at)?),
                "preview" => kind.preview = Some(list(val, at, nonempty)?),
                "search" => kind.search = Some(list(val, at, nonempty)?),
                _ => return Err(format!("{at}: unknown member")),
            }
            Ok(())
        })?;
        Ok(kind)
    }
}

impl Lifecycle {
    pub(crate) fn closed(&self) -> &[String] {
        self.closed.as_deref().unwrap_or_default()
    }

    fn parse(json: Json, path: &str) -> Result<Self, String> {
        let mut lifecycle = Self {
            field: String::new(),
            active: None,
            closed: None,
        };
        members(json, path, |key, at, val| {
            match key.as_str() {
                "field" => lifecycle.field = nonempty(val, at)?,
                "active" => lifecycle.active = Some(list(val, at, string)?),
                "closed" => lifecycle.closed = Some(list(val, at, string)?),
                _ => return Err(format!("{at}: unknown member")),
            }
            Ok(())
        })?;
        if lifecycle.field.is_empty() {
            return Err(format!("{path}.field: missing member"));
        }
        Ok(lifecycle)
    }
}

fn members(
    json: Json,
    path: &str,
    mut visit: impl FnMut(String, &str, Json) -> Result<(), String>,
) -> Result<(), String> {
    let Json::Obj(entries) = json else {
        return Err(format!("{path}: expected an object"));
    };
    let mut seen = HashSet::new();
    for (name, value) in entries {
        let at = format!("{path}.{}", label(&name));
        if !seen.insert(name.clone()) {
            return Err(format!("{at}: repeated member"));
        }
        visit(name, &at, value)?;
    }
    Ok(())
}

fn string(json: Json, path: &str) -> Result<String, String> {
    match json {
        Json::Str(s) => Ok(s),
        _ => Err(format!("{path}: expected text")),
    }
}

fn nonempty(json: Json, path: &str) -> Result<String, String> {
    let s = string(json, path)?;
    if s.is_empty() {
        return Err(format!("{path}: expected nonempty text"));
    }
    Ok(s)
}

fn list(
    json: Json,
    path: &str,
    item: fn(Json, &str) -> Result<String, String>,
) -> Result<Vec<String>, String> {
    let Json::Arr(items) = json else {
        return Err(format!("{path}: expected an array"));
    };
    let mut strings = Vec::with_capacity(items.len());
    let mut seen = HashSet::new();
    for (i, v) in items.into_iter().enumerate() {
        let at = format!("{path}[{i}]");
        let s = item(v, &at)?;
        if !seen.insert(s.clone()) {
            return Err(format!("{at}: repeated list entry"));
        }
        strings.push(s);
    }
    Ok(strings)
}
