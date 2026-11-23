//! Read the reviewed map and the framed, immutable old-store cut.
use loops::json::Json;
use loops_place::line::hash;
use std::{
    collections::{HashMap, HashSet},
    fmt::Write as _,
    io::{self, BufRead},
    str::FromStr,
};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Fold {
    By(String),
    Collect(u64),
}

#[derive(Debug)]
pub(super) struct Map {
    pub source: String,
    pub vertex_sha256: String,
    pub store: String,
    pub kinds: HashMap<String, Fold>,
}

fn object(value: &Json) -> Result<&[(String, Json)], String> {
    if let Json::Obj(fields) = value {
        Ok(fields)
    } else {
        Err("expected an object".into())
    }
}

pub(super) fn fields<'a>(
    value: &'a Json,
    names: &[&str],
) -> Result<HashMap<&'a str, &'a Json>, String> {
    let members = object(value)?;
    let found: HashMap<_, _> = members.iter().map(|(k, v)| (k.as_str(), v)).collect();
    if members.len() != names.len() || !names.iter().all(|name| found.contains_key(name)) {
        return Err(format!("expected exactly the fields {}", names.join(", ")));
    }
    Ok(found)
}

pub(super) fn string(value: &Json) -> Result<&str, String> {
    match value {
        Json::Str(s) => Ok(s),
        _ => Err("expected text".into()),
    }
}

fn decimal<T: FromStr + ToString>(value: &Json) -> Result<T, String> {
    let s = string(value)?;
    s.parse::<T>()
        .ok()
        .filter(|n| n.to_string() == s)
        .ok_or_else(|| "expected a canonical decimal integer in range".into())
}

fn hex(value: &Json, length: usize) -> Result<&str, String> {
    let s = string(value)?;
    if s.len() == length
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(s)
    } else {
        Err(format!("expected {length} lowercase hex digits"))
    }
}

impl Map {
    pub(super) fn read(input: &str) -> Result<Self, String> {
        Self::decode(input).map_err(|reason| format!("shadow-map-invalid: {reason}"))
    }

    fn decode(input: &str) -> Result<Self, String> {
        let json = loops_eval::parse_json(input).map_err(|e| e.to_string())?;
        let data = fields(
            &json,
            &["format", "source", "store", "vertex_sha256", "kinds"],
        )?;
        if string(data["format"])? != "shadow-map-v1" {
            return Err("unknown map version".into());
        }
        let source = string(data["source"])?;
        let store = string(data["store"])?;
        if source.is_empty()
            || matches!(source, "." | "..")
            || source.contains(['/', '\\', '\0'])
            || store.is_empty()
            || store.contains('\0')
        {
            return Err("invalid source or store name".into());
        }
        let mut kinds = HashMap::new();
        for (kind, declaration) in object(data["kinds"])? {
            let fold = match object(declaration)? {
                [(op, Json::Str(key))] if op == "by" && !key.is_empty() => Fold::By(key.clone()),
                [(op, limit)] if op == "collect" => Fold::Collect(decimal(limit)?),
                _ => return Err(format!("invalid fold for {kind:?}")),
            };
            if kind.is_empty() || kinds.insert(kind.clone(), fold).is_some() {
                return Err("empty or duplicate kind".into());
            }
        }
        Ok(Self {
            source: source.into(),
            store: store.into(),
            vertex_sha256: hex(data["vertex_sha256"], 64)?.into(),
            kinds,
        })
    }
}

#[derive(Debug)]
pub(super) struct Fact {
    pub id: String,
    pub coordinate: Vec<i64>,
    pub span: std::ops::Range<u64>,
    pub raw_digest: Vec<u8>,
    pub kind: String,
    pub ts_bits: u64,
    pub observer: String,
    pub origin: String,
    pub signature: Option<String>,
    // Keep the original text for source-prefix verification. Parse only in the
    // row preflight so a bad payload in an omitted kind can be counted.
    pub payload: String,
}

impl Fact {
    fn decode(json: &Json, axes: usize) -> Result<Self, String> {
        let data = fields(
            json,
            &[
                "id",
                "coordinate",
                "kind",
                "ts_bits",
                "observer",
                "origin",
                "signature",
                "payload",
            ],
        )?;
        let id = string(data["id"])?;
        if !(1..=128).contains(&id.len()) || !id.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(
                "expected 1 to 128 printable ASCII characters other than space in old id".into(),
            );
        }
        let Json::Arr(numbers) = data["coordinate"] else {
            return Err("expected a coordinate array".into());
        };
        if numbers.len() != axes {
            return Err("wrong coordinate length".into());
        }
        Ok(Self {
            id: id.into(),
            span: 0..0,
            raw_digest: Vec::new(),
            coordinate: numbers.iter().map(decimal).collect::<Result<_, _>>()?,
            kind: string(data["kind"])?.into(),
            ts_bits: u64::from_str_radix(hex(data["ts_bits"], 16)?, 16).expect("16 hex digits"),
            observer: string(data["observer"])?.into(),
            origin: string(data["origin"])?.into(),
            signature: match data["signature"] {
                Json::Null => None,
                value => Some(string(value)?.into()),
            },
            payload: string(data["payload"])?.into(),
        })
    }

    pub(super) fn parsed_payload(&self) -> Result<Json, loops_eval::Error> {
        loops_eval::parse_writer_payload(&self.payload)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Summary {
    pub facts: u64,
    pub ticks: u64,
}

#[derive(Debug)]
pub(super) enum ScanError {
    Invalid(String),
    Io(io::Error),
}
impl From<String> for ScanError {
    fn from(message: String) -> Self {
        Self::Invalid(message)
    }
}
impl From<&str> for ScanError {
    fn from(message: &str) -> Self {
        Self::Invalid(message.into())
    }
}
impl From<io::Error> for ScanError {
    fn from(error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::InvalidData {
            Self::Invalid(error.to_string())
        } else {
            Self::Io(error)
        }
    }
}
// The digest covers exact UTF-8 bytes, including the terminating LF.
fn line(reader: &mut impl BufRead, buf: &mut String) -> Result<Json, ScanError> {
    buf.clear();
    reader.read_line(buf)?;
    if !buf.ends_with('\n') || buf.ends_with("\r\n") {
        return Err("missing or invalid line ending".into());
    }
    loops_eval::parse_json(buf).map_err(|e| e.to_string().into())
}

/// The callback is for preflight only. Its plans are publishable only after
/// this returns success for every source and no included row was refused.
pub(super) fn scan(
    mut reader: impl BufRead,
    map: &Map,
    mut inspect: impl FnMut(Fact),
) -> Result<Summary, ScanError> {
    let mut number = 1usize;
    let mut run = || {
        let mut buf = String::new();
        let header = line(&mut reader, &mut buf)?;
        let data = fields(
            &header,
            &["format", "source", "vertex_sha256", "ticks", "order"],
        )?;
        if string(data["format"])? != "shadow-facts-v1"
            || string(data["source"])? != map.source
            || string(data["vertex_sha256"])? != map.vertex_sha256
        {
            return Err("export version or source/map binding differs".into());
        }
        let ticks = decimal(data["ticks"])?;
        let axes = match string(data["order"])? {
            "arrival" => 2,
            "rowid" => 1,
            _ => return Err("unknown arrival order".into()),
        };
        let mut digest = hash(buf.as_bytes());
        let mut offset = u64::try_from(buf.len()).expect("line length fits");
        let mut ids = HashSet::new();
        let mut last = Vec::new();
        loop {
            number += 1;
            let record = line(&mut reader, &mut buf)?;
            if object(&record)?.iter().any(|(k, _)| k == "count") {
                let footer = fields(&record, &["count", "digest"])?;
                let facts: u64 = decimal(footer["count"])?;
                let actual = digest.iter().fold(String::new(), |mut s, b| {
                    write!(s, "{b:02x}").expect("write to string");
                    s
                });
                if facts != u64::try_from(ids.len()).expect("count fits")
                    || hex(footer["digest"], 64)? != actual
                {
                    return Err("trailer count or digest differs".into());
                }
                if !reader.fill_buf()?.is_empty() {
                    return Err("bytes after trailer".into());
                }
                return Ok(Summary { facts, ticks });
            }
            let mut fact = Fact::decode(&record, axes)?;
            fact.span = offset..offset + u64::try_from(buf.len()).expect("line length fits");
            offset = fact.span.end;
            fact.raw_digest = hash(buf.as_bytes());
            if fact.coordinate <= last || !ids.insert(fact.id.clone()) {
                return Err("non-increasing coordinate or duplicate old id".into());
            }
            last.clone_from(&fact.coordinate);
            digest.extend_from_slice(buf.as_bytes());
            digest = hash(&digest);
            inspect(fact);
        }
    };
    run().map_err(|error| match error {
        ScanError::Invalid(why) => {
            ScanError::Invalid(format!("shadow-source-invalid at line {number}: {why}"))
        }
        e @ ScanError::Io(_) => e,
    })
}
