//! A checksummed append log. Only a complete, synced reservation permits an append.
use super::{Mode, mismatch};
use crate::{Fail, create_private, text};
use loops::{base32, json::Json};
use loops_place::{key::Key, line::hash};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Pin {
    pub destination: String,
    pub map: String,
    pub unnamed: bool,
}
impl Pin {
    pub(super) fn json(&self, name: &str) -> Json {
        Json::Arr(vec![
            text(name),
            text(&self.destination),
            text(&self.map),
            Json::Bool(self.unnamed),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Seed {
    pub fingerprint: String,
    pub salt: Vec<u8>,
    pub emitted: String,
}
impl Seed {
    pub(super) fn entry(&self, old: &str) -> Json {
        Json::Arr(vec![
            text(old),
            text(&self.fingerprint),
            text(base32::base32(&self.salt)),
            text(&self.emitted),
        ])
    }
}

#[derive(Default)]
pub(super) struct Journal {
    pub public: Option<String>,
    pub pins: HashMap<String, Pin>,
    pub cuts: HashMap<String, (usize, String)>,
    pub omitted: HashMap<String, String>,
    pub seeds: HashMap<String, Seed>,
    pub records: HashMap<String, HashMap<String, (String, usize)>>,
    pub batches: Vec<String>,
    pub done: HashSet<usize>,
    chain: Vec<u8>,
    valid: u64,
    torn: bool,
}
fn bad() -> Fail {
    mismatch("damaged journal")
}
fn word(j: &Json) -> Result<&str, Fail> {
    super::source::string(j).map_err(|_| bad())
}
fn array(j: &Json) -> Result<&[Json], Fail> {
    if let Json::Arr(xs) = j {
        Ok(xs)
    } else {
        Err(bad())
    }
}
fn index(j: &Json) -> Result<usize, Fail> {
    word(j)?.parse().map_err(|_| bad())
}
fn checksum(prev: &[u8], body: &[u8]) -> Vec<u8> {
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(body);
    hash(&bytes)
}

impl Journal {
    pub(super) fn read(path: &Path) -> Result<Self, Fail> {
        let mut journal = Self::default();
        let Some(bytes) = private_read(path)? else {
            return Ok(journal);
        };
        for frame in bytes.split_inclusive(|b| *b == b'\n') {
            let Some(line) = frame.strip_suffix(b"\n") else {
                journal.torn = true;
                break;
            };
            let line = std::str::from_utf8(line).map_err(|_| bad())?;
            let (stamp, body) = line
                .strip_prefix("[\"")
                .and_then(|frame| frame.strip_suffix(']'))
                .and_then(|pair| pair.split_once("\","))
                .ok_or_else(bad)?;
            journal.chain = checksum(&journal.chain, body.as_bytes());
            if base32::base32(&journal.chain) != stamp {
                return Err(bad());
            }
            let event = loops_eval::parse_json(body).map_err(|_| bad())?;
            journal.apply(&event)?;
            journal.valid += u64::try_from(frame.len()).expect("frame length");
        }
        Ok(journal)
    }
    fn apply(&mut self, event: &Json) -> Result<(), Fail> {
        let (op, args) = array(event)?.split_first().ok_or_else(bad)?;
        match (word(op)?, args) {
            ("v1", [_, Json::Arr(_)]) if self.public.is_none() => {
                return Err(mismatch("v1 shadow journal; import into fresh shadows"));
            }
            ("v2", [public, Json::Arr(pins)]) if self.public.is_none() => {
                self.public = Some(word(public)?.into());
                self.add_pins(pins)?;
            }
            ("sources", [Json::Arr(pins)]) if self.public.is_some() => self.add_pins(pins)?,
            ("cut", [name, n, sum, Json::Arr(omitted)]) if self.pins.contains_key(word(name)?) => {
                for row in omitted {
                    let [old, fingerprint] = array(row)? else {
                        return Err(bad());
                    };
                    self.omitted
                        .insert(word(old)?.into(), word(fingerprint)?.into());
                }
                self.cuts
                    .insert(word(name)?.into(), (index(n)?, word(sum)?.into()));
            }
            ("seeds", [Json::Arr(entries)]) if self.public.is_some() => {
                for entry in entries {
                    let [old, fp, salt, time] = array(entry)? else {
                        return Err(bad());
                    };
                    let seed = Seed {
                        fingerprint: word(fp)?.into(),
                        salt: base32::decode(word(salt)?).ok_or_else(bad)?,
                        emitted: word(time)?.into(),
                    };
                    if seed.salt.len() != 16 || self.seeds.insert(word(old)?.into(), seed).is_some()
                    {
                        return Err(bad());
                    }
                }
            }
            ("reserve", [name, Json::Arr(entries)]) if self.pins.contains_key(word(name)?) => {
                let name = word(name)?;
                let batch = self.batches.len();
                let records = self.records.entry(name.into()).or_default();
                for entry in entries {
                    let [old, id] = array(entry)? else {
                        return Err(bad());
                    };
                    let old = word(old)?;
                    if !self.seeds.contains_key(old)
                        || records
                            .insert(old.into(), (word(id)?.into(), batch))
                            .is_some()
                    {
                        return Err(bad());
                    }
                }
                self.batches.push(name.into());
            }
            ("done", [n]) => {
                let batch = index(n)?;
                if batch >= self.batches.len() || !self.done.insert(batch) {
                    return Err(bad());
                }
            }
            _ => return Err(bad()),
        }
        Ok(())
    }
    fn add_pins(&mut self, pins: &[Json]) -> Result<(), Fail> {
        for p in pins {
            let [name, dest, map, Json::Bool(unnamed)] = array(p)? else {
                return Err(bad());
            };
            let pin = Pin {
                destination: word(dest)?.into(),
                map: word(map)?.into(),
                unnamed: *unnamed,
            };
            if self.pins.insert(word(name)?.into(), pin).is_some() {
                return Err(bad());
            }
        }
        Ok(())
    }
    pub(super) fn version(&self) -> (u64, &[u8], bool) {
        (self.valid, &self.chain, self.torn)
    }
    pub(super) fn writer(&self, path: &Path) -> Result<File, Fail> {
        let file = match OpenOptions::new().append(true).open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => create_private(path)?,
            Err(e) => return Err(e.into()),
        };
        if self.torn {
            file.set_len(self.valid)?;
            file.sync_all()?;
        }
        Ok(file)
    }
    pub(super) fn append(&mut self, file: &mut File, event: &Json) -> Result<(), Fail> {
        let body = event.canon();
        let chain = checksum(&self.chain, body.as_bytes());
        let frame = format!("[\"{}\",{body}]\n", base32::base32(&chain));
        self.apply(event)?;
        file.write_all(frame.as_bytes())?;
        file.sync_data()?;
        self.chain = chain;
        self.valid += u64::try_from(frame.len()).expect("frame length");
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Private {
    Directory,
    File,
}
fn inspect(path: &Path, kind: Private) -> Result<bool, Fail> {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    if !(if kind == Private::Directory {
        meta.is_dir()
    } else {
        meta.is_file()
    }) {
        return Err(mismatch("invalid private state path"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o777
            != if kind == Private::Directory {
                0o700
            } else {
                0o600
            }
        {
            return Err(mismatch("unsafe private state permissions"));
        }
    }
    Ok(true)
}
fn private_read(path: &Path) -> Result<Option<Vec<u8>>, Fail> {
    if inspect(path, Private::File)? {
        Ok(Some(fs::read(path)?))
    } else {
        Ok(None)
    }
}
pub(super) fn state_exists(dir: &Path) -> Result<bool, Fail> {
    inspect(dir, Private::Directory)
}
pub(super) fn ensure_dir(dir: &Path) -> Result<(), Fail> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.recursive(true).create(dir) {
        Ok(()) => (),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e.into()),
    }
    inspect(dir, Private::Directory)?;
    Ok(())
}
pub(super) fn key(dir: &Path) -> Result<Option<Key>, Fail> {
    private_read(&dir.join("key"))?
        .map(|bytes| {
            std::str::from_utf8(&bytes)
                .ok()
                .and_then(Key::from_text)
                .ok_or_else(|| mismatch("invalid import key"))
        })
        .transpose()
}
pub(super) fn save_key(dir: &Path, key: &Key) -> Result<(), Fail> {
    // A stopped write leaves only a scratch file. The group lock owns it.
    let temp = dir.join("key.partial");
    if inspect(&temp, Private::File)? {
        fs::remove_file(&temp)?;
    }
    let mut f = create_private(&temp)?;
    writeln!(f, "{}", key.text())?;
    f.sync_all()?;
    fs::rename(temp, dir.join("key"))?;
    Ok(())
}

// flock is available on the POSIX hosts supported by this private store.
// Holding the descriptor holds the lock, including across a segment rename.
// Closing it (also on process death) releases it without a stale-lock protocol.
#[cfg(unix)]
pub(super) fn exclusive(f: &File) -> Result<(), Fail> {
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn flock(fd: std::ffi::c_int, operation: std::ffi::c_int) -> std::ffi::c_int;
    }
    const LOCK_EX: std::ffi::c_int = 2;
    const LOCK_NB: std::ffi::c_int = 4;
    // SAFETY: the borrowed descriptor remains open through this synchronous call.
    if unsafe { flock(f.as_raw_fd(), LOCK_EX | LOCK_NB) } != 0 {
        let e = io::Error::last_os_error();
        if e.kind() == io::ErrorKind::WouldBlock {
            return Err(mismatch("another import holds the lock"));
        }
        return Err(e.into());
    }
    Ok(())
}
#[cfg(not(unix))]
pub(super) fn exclusive(_: &File) -> Result<(), Fail> {
    Err(Fail::System(
        "private shadow stores require POSIX permissions and locks".into(),
    ))
}
pub(super) fn lock(dir: &Path, mode: Mode) -> Result<Option<File>, Fail> {
    let path = dir.join("lock");
    let f = if inspect(&path, Private::File)? {
        File::open(&path)?
    } else if mode == Mode::Write {
        match create_private(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => File::open(&path)?,
            Err(e) => return Err(e.into()),
        }
    } else {
        return Ok(None);
    };
    exclusive(&f)?;
    Ok(Some(f))
}
