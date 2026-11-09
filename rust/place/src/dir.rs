//! A directory place: immutable segments of stored lines.
//!
//! ```text
//! <root>/segments/<key>/<hash>.jsonl
//! ```
//!
//! A segment holds the lines one key signed in one changeset, and is named by
//! the SHA-256 of its bytes. Writing one is a rename into place, so a reader
//! sees a whole changeset or none of it. Nothing appends to a segment, so two
//! copies of a store merge by adding files, and the store they hold is the
//! union of their records.
//!
//! The only write that removes bytes is [`Dir::purge`], which replaces a
//! segment by the same lines with openings erased. A segment whose name is
//! not the hash of its bytes changed some other way; [`Held::altered`] lists
//! those.
use crate::line::{Line, Refusal, hash};
use loops::{base32, record::Record, things};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

/// A directory that holds a store's segments.
#[derive(Clone, Debug)]
pub struct Dir {
    root: PathBuf,
}

/// A line a segment holds that did not check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    /// The segment, as a path below the place's root.
    pub segment: String,
    /// The line's number in the segment, from 1.
    pub line: usize,
    /// The check that failed.
    pub refusal: Refusal,
}

/// Everything a place holds, read once.
#[derive(Clone, Debug, Default)]
pub struct Held {
    lines: Vec<Line>,
    refused: Vec<Refused>,
    altered: Vec<String>,
    segments: usize,
}

fn name_of(bytes: &[u8]) -> String {
    format!("{}.jsonl", base32::base32(&hash(bytes)))
}
fn bytes_of(lines: &[&Line]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in lines {
        bytes.extend_from_slice(line.render().as_bytes());
        bytes.push(b'\n');
    }
    bytes
}
fn key_dir(key: &str) -> &str {
    key.strip_prefix("k:").unwrap_or(key)
}

impl Dir {
    /// Makes an empty place at `root`, or opens the one already there.
    ///
    /// # Errors
    /// Returns the file system's error.
    pub fn init(root: &Path) -> io::Result<Self> {
        fs::create_dir_all(root.join("segments"))?;
        Self::open(root)
    }
    /// Opens the place at `root`.
    ///
    /// # Errors
    /// Returns `NotFound` when `root` holds no `segments` directory.
    pub fn open(root: &Path) -> io::Result<Self> {
        if fs::metadata(root.join("segments"))?.is_dir() {
            Ok(Self {
                root: root.to_owned(),
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "segments is not a directory",
            ))
        }
    }
    /// The directory this place lives in.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn segments(&self) -> io::Result<Vec<PathBuf>> {
        let mut found = Vec::new();
        for entry in fs::read_dir(self.root.join("segments"))? {
            let dir = entry?.path();
            if !dir.is_dir() {
                continue;
            }
            for entry in fs::read_dir(&dir)? {
                let path = entry?.path();
                let hidden = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_none_or(|n| n.starts_with('.'));
                if !hidden && path.extension().is_some_and(|e| e == "jsonl") {
                    found.push(path);
                }
            }
        }
        found.sort();
        Ok(found)
    }
    fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }

    /// Reads every segment.
    ///
    /// A last line without its line ending is a write that never finished, and
    /// is skipped. A line that does not check is listed, not held.
    ///
    /// # Errors
    /// Returns the file system's error.
    pub fn read(&self) -> io::Result<Held> {
        let mut held = Held::default();
        for path in self.segments()? {
            let bytes = fs::read(&path)?;
            let segment = self.relative(&path);
            held.segments += 1;
            if path.file_name().and_then(|n| n.to_str()) != Some(name_of(&bytes).as_str()) {
                held.altered.push(segment.clone());
            }
            let text = String::from_utf8_lossy(&bytes);
            let mut lines: Vec<&str> = text.split('\n').collect();
            lines.pop();
            for (i, line) in lines.into_iter().enumerate() {
                match Line::parse(line) {
                    Ok(line) => held.lines.push(line),
                    Err(refusal) => held.refused.push(Refused {
                        segment: segment.clone(),
                        line: i + 1,
                        refusal,
                    }),
                }
            }
        }
        Ok(held)
    }

    fn write_segment(&self, key: &str, bytes: &[u8]) -> io::Result<PathBuf> {
        let dir = self.root.join("segments").join(key_dir(key));
        fs::create_dir_all(&dir)?;
        let name = name_of(bytes);
        let path = dir.join(&name);
        if !path.exists() {
            let partial = dir.join(format!(".{name}.partial"));
            let mut file = fs::File::create(&partial)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            fs::rename(&partial, &path)?;
        }
        Ok(path)
    }

    /// Writes one changeset: a closed segment for each key that signed part of it.
    /// Writing the same changeset twice changes nothing.
    ///
    /// # Errors
    /// Returns the file system's error. A changeset signed by several keys is
    /// several renames, so a failure can leave some of its segments written.
    pub fn append(&self, lines: &[Line]) -> io::Result<Vec<String>> {
        let mut keys: Vec<&str> = lines.iter().map(Line::key).collect();
        keys.sort_unstable();
        keys.dedup();
        keys.into_iter()
            .map(|key| {
                let own: Vec<&Line> = lines.iter().filter(|l| l.key() == key).collect();
                self.write_segment(key, &bytes_of(&own))
                    .map(|path| self.relative(&path))
            })
            .collect()
    }

    /// Removes the openings of the named records from this place, keeping
    /// each commitment, id and signature. Returns how many openings went.
    ///
    /// # Errors
    /// Returns the file system's error. Segments are replaced one at a time.
    pub fn purge(&self, ids: &[String]) -> io::Result<usize> {
        let mut gone = 0;
        for path in self.segments()? {
            let bytes = fs::read(&path)?;
            let text = String::from_utf8_lossy(&bytes);
            let mut lines: Vec<&str> = text.split('\n').collect();
            lines.pop();
            let Ok(parsed) = lines
                .iter()
                .map(|l| Line::parse(l))
                .collect::<Result<Vec<_>, _>>()
            else {
                continue;
            };
            let targets = parsed
                .iter()
                .filter(|l| l.payload().is_some() && ids.iter().any(|id| id == l.id()))
                .count();
            if targets == 0 {
                continue;
            }
            let erased: Vec<Line> = parsed
                .iter()
                .map(|l| {
                    if ids.iter().any(|id| id == l.id()) {
                        l.erased()
                    } else {
                        l.clone()
                    }
                })
                .collect();
            let Some(key) = erased.first().map(Line::key) else {
                continue;
            };
            let written = self.write_segment(key, &bytes_of(&erased.iter().collect::<Vec<_>>()))?;
            if written != path {
                fs::remove_file(&path)?;
            }
            gone += targets;
        }
        Ok(gone)
    }
}

impl Held {
    /// Every checked line, in segment order. An id can appear more than once,
    /// with or without its opening.
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }
    /// Lines that did not check.
    pub fn refused(&self) -> &[Refused] {
        &self.refused
    }
    /// Segments whose name is not the hash of their bytes.
    pub fn altered(&self) -> &[String] {
        &self.altered
    }
    /// How many segments were read.
    pub fn segments(&self) -> usize {
        self.segments
    }
    /// One record per held id, settled: a held opening wins over a bare
    /// commitment, and a held eraser erases it.
    pub fn settled(&self) -> Vec<Record> {
        things::settle(&self.lines.iter().map(Line::record).collect::<Vec<_>>())
    }
    /// The line for an id, with its opening when any segment holds it.
    pub fn line(&self, id: &str) -> Option<&Line> {
        let mut forms = self.lines.iter().filter(|l| l.id() == id);
        let first = forms.next()?;
        if first.payload().is_some() {
            return Some(first);
        }
        Some(forms.find(|l| l.payload().is_some()).unwrap_or(first))
    }
    /// The ids that start with `prefix`, each once, in order. A prefix may
    /// leave off the `r:`.
    pub fn ids_from(&self, prefix: &str) -> Vec<&str> {
        let prefix = prefix.strip_prefix("r:").unwrap_or(prefix);
        let mut ids: Vec<&str> = self
            .lines
            .iter()
            .map(Line::id)
            .filter(|id| id[2..].starts_with(prefix))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }
}
