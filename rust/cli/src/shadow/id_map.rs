//! The salt-free index published beside the group after all segments are complete.
use super::{Input, Point, Runtime, invalid, mapping::Prepared};
use crate::{Fail, create_private, object, text};
use loops::json::Json;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Default)]
pub(super) struct IdMap(BTreeMap<String, BTreeMap<String, Json>>);

impl IdMap {
    pub(super) fn seen(&mut self, id: &str) {
        self.0.entry(id.into()).or_default();
    }
    pub(super) fn record(&mut self, old: &str, input: &Input, p: &Prepared) {
        self.0.entry(old.into()).or_default().insert(
            input.name.clone(),
            object(vec![
                ("source", text(&input.name)),
                ("destination", text(&input.relative_destination)),
                ("record", text(&p.id)),
                ("thing", text(p.draft.env.of.as_ref().unwrap_or(&p.id))),
            ]),
        );
    }
    pub(super) fn publish(self, path: &Path, runtime: &Runtime) -> Result<(), Fail> {
        let rows = self
            .0
            .into_iter()
            .map(|(old, entries)| (old, Json::Arr(entries.into_values().collect())))
            .collect();
        let bytes = format!("{}\n", Json::Obj(rows).canon());
        match fs::read(path) {
            Ok(existing) if existing == bytes.as_bytes() => return Ok(()),
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
        let parent = path.parent().expect("group parent");
        let (temp, mut file) = (0..)
            .find_map(|n| {
                let temp = parent.join(format!(".shadow-ids-{}-{n}.partial", std::process::id()));
                match create_private(&temp) {
                    Ok(file) => Some(Ok((temp, file))),
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => None,
                    Err(e) => Some(Err(e)),
                }
            })
            .expect("temporary name available")?;
        let result = (|| {
            file.write_all(bytes.as_bytes())?;
            file.sync_all()?;
            (runtime.at)(Point::BeforeMapRename)?;
            fs::rename(&temp, path)?;
            File::open(parent)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result
    }
}

pub(super) fn relative_destination(parent: &Path, directory: &Path) -> Result<String, Fail> {
    let common = parent
        .ancestors()
        .find(|p| directory.starts_with(p))
        .expect("absolute paths share root");
    let mut path = PathBuf::new();
    for _ in parent
        .strip_prefix(common)
        .expect("common ancestor")
        .components()
    {
        path.push("..");
    }
    path.push(directory.strip_prefix(common).expect("common ancestor"));
    if path.as_os_str().is_empty() {
        path.push(".");
    }
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid("destination path is not UTF-8"))
}
