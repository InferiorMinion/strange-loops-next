//! Checked records and descriptions for reads across a selection of open stores.
use crate::{Description, Fail, registry::SelectedStore, unclean};
use loops::json::Json;
use loops_place::dir::Held;
use std::ops::Range;

pub(crate) struct Site {
    pub(crate) directory: String,
    pub(crate) description: Option<(Description, Json)>,
    pub(crate) lines: Range<usize>,
}

pub(crate) struct Union {
    pub(crate) held: Held,
    pub(crate) sites: Vec<Site>,
    pub(crate) integrity: Option<String>,
}

impl Union {
    pub(crate) fn read(stores: Vec<SelectedStore>) -> Result<Self, Fail> {
        let mut held = Held::default();
        let mut dirty = Vec::new();
        let mut sites = Vec::new();
        for SelectedStore { directory, store } in stores {
            let directory = directory
                .to_str()
                .ok_or_else(|| {
                    Fail::System(format!(
                        "store directory is not UTF-8: {}",
                        directory.display()
                    ))
                })?
                .to_owned();
            let description = Description::with_source(store.dir.root())?;
            let local = store.read()?;
            if unclean(&local).is_some() {
                dirty.push(directory.clone());
            }
            let start = held.lines().len();
            held.append(local);
            sites.push(Site {
                directory,
                description,
                lines: start..held.lines().len(),
            });
        }
        let integrity = unclean(&held).map(|why| format!("{why} (stores: {})", dirty.join(", ")));
        Ok(Self {
            held,
            sites,
            integrity,
        })
    }
}
