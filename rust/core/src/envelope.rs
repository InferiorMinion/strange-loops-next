//! Env validation and canonical wire data from model/Loops/Envelope.lean.
use crate::read::{members, optional};
use crate::{
    json::{self, Json},
    occurred,
    stored::Error,
    time,
};

/// Lean: `Loops.shapeCodes`, model/Loops/Envelope.lean.
/// A raw envelope shape refusal, checked across every envelope before any value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeError {
    /// A raw envelope has an unknown field.
    FieldUnknown,
    /// A raw envelope is not an object, or a draft has none.
    NotObject,
}

impl ShapeError {
    /// Returns the exact wire code.
    pub fn code(self) -> &'static str {
        match self {
            Self::FieldUnknown => "envelope-field-unknown",
            Self::NotObject => "envelope-not-object",
        }
    }
}

impl std::fmt::Display for ShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for ShapeError {}

/// Lean: `Loops.shapeCodes`, model/Loops/Envelope.lean.
/// The wire codes of raw shape, in the order they are checked.
pub const SHAPE_CODES: [&str; 2] = ["envelope-field-unknown", "envelope-not-object"];

/// Checks raw envelope member names without inspecting values.
///
/// # Errors
/// Returns `ShapeError::FieldUnknown` if any member name is unknown.
pub fn check_keys<'a>(keys: impl IntoIterator<Item = &'a str>) -> Result<(), ShapeError> {
    const KEYS: &[&str] = &[
        "kind", "observer", "emitted", "occurred", "about", "of", "prev", "under", "erases",
    ];
    if keys.into_iter().any(|key| !KEYS.contains(&key)) {
        Err(ShapeError::FieldUnknown)
    } else {
        Ok(())
    }
}

/// Lean: `Loops.envelopesShapeProblem`, model/Loops/Envelope.lean.
/// Checks all raw envelopes before any value decoding: an unknown key in any
/// object first, then any envelope that is not an object.
///
/// # Errors
/// Returns the first shape refusal across every envelope.
pub fn check_all_shapes<'a>(envs: impl IntoIterator<Item = &'a Json>) -> Result<(), ShapeError> {
    let envs: Vec<_> = envs.into_iter().collect();
    for env in &envs {
        if let Json::Obj(fields) = env {
            check_keys(fields.iter().map(|(key, _)| key.as_str()))?;
        }
    }
    if envs.iter().all(|env| matches!(env, Json::Obj(_))) {
        Ok(())
    } else {
        Err(ShapeError::NotObject)
    }
}

/// Decodes one raw envelope, checking its shape before its values.
///
/// # Errors
/// Returns a raw input error for a shape refusal or repeated member names.
pub fn read(path: &str, j: &Json) -> Result<Env, crate::read::Error> {
    check_all_shapes([j]).map_err(|e| crate::read::Error::from(e.to_string()))?;
    decode_values(path, j)
}

/// Decodes variants. A singleton uses the one-envelope boundary directly.
///
/// # Errors
/// Returns a raw input error for a shape refusal or repeated member names.
pub fn read_all(path: &str, j: &Json) -> Result<Vec<Env>, crate::read::Error> {
    let raw = crate::read::list(path, j)?;
    if let [one] = raw {
        return Ok(vec![read(&format!("{path}[0]"), one)?]);
    }
    check_all_shapes(raw).map_err(|e| crate::read::Error::from(e.to_string()))?;
    raw.iter()
        .enumerate()
        .map(|(i, j)| decode_values(&format!("{path}[{i}]"), j))
        .collect()
}

/// Lean: `Loops.unreadable`, model/Loops/Envelope.lean.
/// What the reader reads for a field it can't read as its JSON type: empty
/// text, which every field's own check refuses, so the refusal is the field's
/// code in the field's place.
pub const UNREADABLE: &str = "";

/// Lean: `Loops.Read.textOr`, model/Loops/Read.lean.
fn text_or(j: &Json) -> String {
    match j {
        Json::Str(s) => s.clone(),
        _ => UNREADABLE.to_owned(),
    }
}

/// Lean: `Loops.Read.textsOr`, model/Loops/Read.lean.
/// A set that is not an array reads as one unreadable member; a member that
/// is not text reads as unreadable in its place.
fn texts_or(j: &Json) -> Vec<String> {
    match j {
        Json::Arr(xs) => xs.iter().map(text_or).collect(),
        _ => vec![UNREADABLE.to_owned()],
    }
}

/// Lean: `Loops.Read.decodeValues`, model/Loops/Read.lean.
/// A required field left out, or any field of the wrong JSON type, reads as
/// `UNREADABLE`, which that field's check refuses. Repeated member names are
/// still malformed input.
pub(crate) fn decode_values(path: &str, j: &Json) -> Result<Env, crate::read::Error> {
    let Json::Obj(raw) = j else {
        return Err(crate::read::Error::from(
            ShapeError::NotObject.code().to_owned(),
        ));
    };
    let names: Vec<_> = raw.iter().map(|(k, _)| k.as_str()).collect();
    let kvs = members(path, &names, j)?;
    let required = |k: &str| optional(kvs, k).map_or_else(|| UNREADABLE.to_owned(), text_or);
    let set = |k: &str| optional(kvs, k).map(texts_or).unwrap_or_default();
    Ok(Env {
        kind: required("kind"),
        observer: required("observer"),
        emitted: required("emitted"),
        occurred: optional(kvs, "occurred").map(text_or),
        about: set("about"),
        of: optional(kvs, "of").map(text_or),
        prev: set("prev"),
        under: optional(kvs, "under").map(text_or),
        erases: set("erases"),
    })
}

/// Lean: `Loops.Read.salt`, model/Loops/Read.lean.
/// A salt written in base32hex. One left out, not text, or not canonical
/// base32hex reads as no bytes, which the writer refuses as `salt-not-16-bytes`.
pub fn read_salt(j: Option<&Json>) -> Vec<u8> {
    match j {
        Some(Json::Str(s)) => crate::base32::decode(s).unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Lean: `Loops.Env`, model/Loops/Envelope.lean.
/// An unchecked writer envelope keeps text spellings and raw lists.
/// Only `to_json` sorts its set fields and removes repeats.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Env {
    /// The name for the record's kind.
    pub kind: String,
    /// The typed reference for the observer who emits the record.
    pub observer: String,
    /// The observer's current time in UTC.
    pub emitted: String,
    /// The observer's claim about past time, if any.
    pub occurred: Option<String>,
    /// The typed references for the things this record mentions.
    pub about: Vec<String>,
    /// The typed reference for the one thing this record is an emit of, if it says.
    /// Only a root leaves it out: a record with `prev` states it (`of-missing`).
    pub of: Option<String>,
    /// The text ids of the records this record supersedes.
    pub prev: Vec<String>,
    /// The text id of the grant this record uses, if any.
    pub under: Option<String>,
    /// The text ids of the records whose payloads this record erases.
    pub erases: Vec<String>,
}
/// Lean: `Loops.typedRef`, model/Loops/Envelope.lean.
/// Checks a lowercase ASCII prefix and a nonempty value after the first colon.
pub fn typed_ref(s: &str) -> bool {
    let Some((prefix, value)) = s.split_once(':') else {
        return false;
    };
    let mut chars = prefix.bytes();
    !value.is_empty()
        && chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
/// Lean: `Loops.recordId`, model/Loops/Envelope.lean.
/// Checks the `r:` prefix and canonical encoding of a 32-byte digest.
pub fn record_id(s: &str) -> bool {
    s.strip_prefix("r:")
        .and_then(crate::base32::decode)
        .is_some_and(|bytes| bytes.len() == 32)
}
impl Env {
    /// Lean: `Loops.Env.toJson`, model/Loops/Envelope.lean.
    /// Writes the envelope, sorts its set fields, and removes repeats.
    /// Omits empty sets and optional fields with no value.
    pub fn to_json(&self) -> Json {
        let mut xs = vec![
            ("kind".into(), Json::Str(self.kind.clone())),
            ("observer".into(), Json::Str(self.observer.clone())),
            ("emitted".into(), Json::Str(self.emitted.clone())),
        ];
        if let Some(s) = &self.occurred {
            xs.push(("occurred".into(), Json::Str(s.clone())));
        }
        let set = |k: &str, values: &[String], xs: &mut Vec<(String, Json)>| {
            if !values.is_empty() {
                xs.push((
                    k.into(),
                    Json::Arr(json::canon_set(values).into_iter().map(Json::Str).collect()),
                ));
            }
        };
        set("about", &self.about, &mut xs);
        if let Some(s) = &self.of {
            xs.push(("of".into(), Json::Str(s.clone())));
        }
        set("prev", &self.prev, &mut xs);
        if let Some(s) = &self.under {
            xs.push(("under".into(), Json::Str(s.clone())));
        }
        if !self.erases.is_empty() {
            xs.push((
                "erases".into(),
                Json::Arr(
                    json::canon_set(&self.erases)
                        .into_iter()
                        .map(Json::Str)
                        .collect(),
                ),
            ));
        }
        Json::Obj(xs)
    }
    /// Lean: `Loops.Env.problem`, model/Loops/Envelope.lean.
    /// Returns the first envelope refusal in protocol check order.
    pub fn problem(&self) -> Option<Error> {
        if self.kind.is_empty() {
            Some(Error::KindEmpty)
        } else if !typed_ref(&self.observer) {
            Some(Error::ObserverNotTyped)
        } else if !time::valid_emitted(&self.emitted) {
            Some(Error::EmittedNotRfc3339)
        } else if self
            .occurred
            .as_deref()
            .is_some_and(|s| occurred::occurred_span(s).is_none())
        {
            Some(Error::OccurredNotSpan)
        } else if !self.about.iter().all(|s| typed_ref(s)) {
            Some(Error::AboutNotTyped)
        } else if self.of.as_deref().is_some_and(|s| !typed_ref(s)) {
            Some(Error::OfNotTyped)
        } else if !self.prev.iter().all(|s| record_id(s)) {
            Some(Error::PrevNotRecordId)
        } else if self.under.as_deref().is_some_and(|s| !record_id(s)) {
            Some(Error::UnderNotRecordId)
        } else if !self.erases.iter().all(|s| record_id(s)) {
            Some(Error::ErasesNotRecordId)
        } else if !self.prev.is_empty() && self.of.is_none() {
            Some(Error::OfMissing)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod raw_tests {
    use super::*;

    #[test]
    fn a_nonobject_envelope_and_a_missing_or_mistyped_field_refuse_by_name() {
        let good = || {
            vec![
                ("kind".to_owned(), Json::Str("note".into())),
                ("observer".to_owned(), Json::Str("o:test".into())),
                (
                    "emitted".to_owned(),
                    Json::Str("2026-10-21T00:00:00Z".into()),
                ),
            ]
        };
        let unknown = Json::Obj(vec![("extra".into(), Json::Null)]);
        for raw in [
            Json::Null,
            Json::Int(7_i64.into()),
            Json::Str("note".into()),
            Json::Arr(vec![]),
        ] {
            assert_eq!(
                read("env", &raw).expect_err("shape").to_string(),
                "envelope-not-object"
            );
            for envs in [
                vec![Json::Obj(good()), raw.clone()],
                vec![raw.clone(), Json::Obj(good())],
            ] {
                assert_eq!(
                    read_all("envs", &Json::Arr(envs))
                        .expect_err("shape")
                        .to_string(),
                    "envelope-not-object"
                );
            }
            for envs in [
                vec![unknown.clone(), raw.clone()],
                vec![raw.clone(), unknown.clone()],
            ] {
                assert_eq!(
                    read_all("envs", &Json::Arr(envs))
                        .expect_err("shape")
                        .to_string(),
                    "envelope-field-unknown"
                );
            }
        }
        let fields = [
            ("kind", Error::KindEmpty, true),
            ("observer", Error::ObserverNotTyped, true),
            ("emitted", Error::EmittedNotRfc3339, true),
            ("occurred", Error::OccurredNotSpan, false),
            ("about", Error::AboutNotTyped, false),
            ("of", Error::OfNotTyped, false),
            ("prev", Error::PrevNotRecordId, false),
            ("under", Error::UnderNotRecordId, false),
            ("erases", Error::ErasesNotRecordId, false),
        ];
        for (key, code, required) in fields {
            let wrong = [
                Json::Null,
                Json::Bool(true),
                Json::Int(7_i64.into()),
                Json::Obj(vec![]),
                Json::Arr(vec![Json::Int(7_i64.into())]),
            ];
            let mut values: Vec<Option<Json>> = wrong.into_iter().map(Some).collect();
            if required {
                values.push(None);
            }
            for value in values {
                let mut kvs: Vec<_> = good().into_iter().filter(|(k, _)| k != key).collect();
                if let Some(v) = value.clone() {
                    kvs.push((key.to_owned(), v));
                }
                let env = read("env", &Json::Obj(kvs)).expect("an object envelope reads");
                assert_eq!(env.problem(), Some(code), "{key}: {value:?}");
            }
        }
        assert_eq!(read_salt(None), Vec::<u8>::new());
        assert_eq!(read_salt(Some(&Json::Int(7_i64.into()))), Vec::<u8>::new());
        assert_eq!(read_salt(Some(&Json::Str("!".into()))), Vec::<u8>::new());
        assert_eq!(read_salt(Some(&Json::Str("00".into()))), vec![0]);
    }

    #[test]
    fn raw_shape_precedes_singleton_and_variant_values() {
        let unknown = Json::Obj(vec![("extra".into(), Json::Null)]);
        let number_kind = Json::Obj(vec![("kind".into(), Json::Int(7_i64.into()))]);
        for raw in [
            unknown.clone(),
            Json::Obj(vec![
                ("kind".into(), Json::Int(7_i64.into())),
                ("extra".into(), Json::Null),
            ]),
        ] {
            assert_eq!(
                read("env", &raw).expect_err("shape").to_string(),
                "envelope-field-unknown"
            );
            assert_eq!(
                read_all("envs", &Json::Arr(vec![raw]))
                    .expect_err("singleton shape")
                    .to_string(),
                "envelope-field-unknown"
            );
        }
        for malformed in [Json::Null, number_kind] {
            for raw in [
                vec![malformed.clone(), unknown.clone()],
                vec![unknown.clone(), malformed.clone()],
            ] {
                assert_eq!(
                    read_all("envs", &Json::Arr(raw))
                        .expect_err("all shape")
                        .to_string(),
                    "envelope-field-unknown"
                );
            }
        }
        let duplicate = Json::Obj(vec![
            ("extra".into(), Json::Null),
            ("extra".into(), Json::Null),
        ]);
        assert_eq!(
            read("env", &duplicate)
                .expect_err("raw shape first")
                .to_string(),
            "envelope-field-unknown"
        );
    }
}
