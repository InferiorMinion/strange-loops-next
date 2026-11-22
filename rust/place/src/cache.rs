//! Local evidence of a successful whole-segment check. Every cache error is a miss.
use loops::base32;
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write},
    path::{Component, Path, PathBuf},
};

/// Bump when tightening any check required to remember a segment.
const LINE_CHECK_VERSION: u32 = 1;

// Resolve existing ancestors before making any directories. An override or
// symlink pointing into a store must never let the store vouch for its bytes.
fn outside_store(dir: &Path, store: &Path) -> Option<PathBuf> {
    if !dir.is_absolute()
        || dir
            .components()
            .any(|c| c == Component::ParentDir || c.as_os_str() == ".loops")
    {
        return None;
    }
    for parent in dir.ancestors() {
        match parent.canonicalize() {
            Ok(existing) => {
                let dest = existing.join(dir.strip_prefix(parent).ok()?);
                if dest.starts_with(store.canonicalize().ok()?)
                    || dest.components().any(|c| c.as_os_str() == ".loops")
                {
                    return None;
                }
                return Some(dest);
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return None,
        }
    }
    None
}

// A partial line or one malformed entry invalidates the whole list.
fn parse_list(text: &str) -> Option<BTreeSet<String>> {
    text.split_inclusive('\n')
        .map(|line| {
            let digest = line.strip_suffix('\n')?;
            (digest.len() == 52 && base32::decode(digest)?.len() == 32).then(|| digest.to_owned())
        })
        .collect()
}

pub(crate) struct Checked {
    path: PathBuf,
    hashes: BTreeSet<String>,
    dirty: bool,
}

impl Checked {
    pub(crate) fn open(dir: &Path, store: &Path) -> Option<Self> {
        let dir = outside_store(dir, store)?;
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.recursive(true).create(&dir).ok()?;
        let meta = fs::metadata(&dir).ok()?;
        if !meta.is_dir() || meta.permissions().readonly() {
            return None;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if meta.mode() & 0o022 != 0 {
                return None;
            }
        }
        let path = dir.join(format!("checked-segments-v{LINE_CHECK_VERSION}"));
        let hashes = match fs::symlink_metadata(&path) {
            Ok(file) => {
                if !file.is_file() || file.permissions().readonly() {
                    return None;
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if file.mode() & 0o077 != 0 || file.uid() != meta.uid() {
                        return None;
                    }
                }
                let bytes = fs::read(&path).ok()?;
                std::str::from_utf8(&bytes)
                    .ok()
                    .and_then(parse_list)
                    .unwrap_or_default()
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => BTreeSet::new(),
            Err(_) => return None,
        };
        Some(Self {
            path,
            hashes,
            dirty: false,
        })
    }

    pub(crate) fn contains(&self, digest: &str) -> bool {
        self.hashes.contains(digest)
    }

    pub(crate) fn remember(&mut self, digest: String) {
        self.dirty |= self.hashes.insert(digest);
    }

    pub(crate) fn save(&self) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let mut nonce = [0; 16];
        getrandom::getrandom(&mut nonce).map_err(|e| io::Error::other(e.to_string()))?;
        let tmp = self
            .path
            .with_file_name(format!(".checked-{}.tmp", base32::base32(&nonce)));
        let mut options = fs::OpenOptions::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.create_new(true).write(true).open(&tmp)?;
        let result = (|| {
            let mut out = io::BufWriter::new(file);
            for digest in &self.hashes {
                writeln!(out, "{digest}")?;
            }
            out.flush()?;
            drop(out);
            fs::rename(&tmp, &self.path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(tmp);
        }
        result
    }
}
