//! The stored line of `protocol/record.md`: one signed record as canonical JSON.
//!
//! A [`Line`] exists only checked. Reading one recomputes its id from the
//! envelope and the commitment, verifies the signature over that id, and
//! refuses any spelling but the canonical one. So holding a `Line` means the
//! bytes say what they claim, and nothing about who was allowed to write them.
use crate::key::Key;
use ed25519_dalek::{Signature, VerifyingKey};
use loops::{
    base32,
    envelope::{self, Env},
    json::Json,
    occurred::{Claim, Stamp},
    record::{Digest, Record, Val},
    stored::{self, Body, Stored, Written},
};
use sha2::{Digest as _, Sha256};
use std::fmt;

/// SHA-256, the hash layer 0 names.
pub fn hash(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

/// Why a line is not held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The text is not one JSON value.
    NotJson,
    /// The members are not those of a stored line, or one has the wrong type.
    Shape,
    /// The record fails a writer check, named by its layer 0 code.
    Write(stored::Error),
    /// The commitment is not `h:` and 32 bytes of base32hex.
    CommitInvalid,
    /// The id is not the hash of the envelope and the commitment.
    IdMismatch,
    /// The key is not `k:` and 32 bytes of base32hex naming a curve point.
    KeyInvalid,
    /// The signature is not the key's signature over the id.
    SignatureInvalid,
    /// The record checks, but the line is not its canonical spelling.
    NotCanonical,
}
impl Refusal {
    /// The refusal's code. Writer checks keep their layer 0 codes.
    pub fn code(self) -> &'static str {
        match self {
            Self::NotJson => "line-not-json",
            Self::Shape => "line-shape",
            Self::Write(error) => error.code(),
            Self::CommitInvalid => "commit-invalid",
            Self::IdMismatch => "id-mismatch",
            Self::KeyInvalid => "key-invalid",
            Self::SignatureInvalid => "signature-invalid",
            Self::NotCanonical => "line-not-canonical",
        }
    }
}
impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Refusal {}

/// One checked stored line: a record, its id, and a signature over that id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    id: String,
    stored: Stored,
    key: String,
    sig: String,
}

fn member<'a>(members: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
    members.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}
fn text<'a>(members: &'a [(String, Json)], name: &str) -> Result<&'a str, Refusal> {
    match member(members, name) {
        Some(Json::Str(s)) => Ok(s),
        _ => Err(Refusal::Shape),
    }
}
fn names_are(members: &[(String, Json)], names: &[&str]) -> bool {
    members.len() == names.len() && names.iter().all(|n| member(members, n).is_some())
}
fn digest(id: &str) -> Digest {
    let bytes = id
        .strip_prefix("r:")
        .and_then(base32::decode)
        .expect("a held line's ids passed the envelope's record id check");
    Digest::try_from(bytes.as_slice()).expect("a record id is 32 bytes")
}

impl Line {
    /// Signs a checked record with a key.
    pub fn sign(written: &Written, key: &Key) -> Self {
        let stored = written.stored();
        let id = stored.id(&hash);
        let sig = base32::base32(&key.sign(&stored::signing_input(&id)));
        Self {
            id,
            stored,
            key: key.public(),
            sig,
        }
    }

    /// Reads one line, checking everything its bytes can prove.
    ///
    /// # Errors
    /// Returns the first check that fails: JSON, shape, the writer's checks,
    /// the commitment, the id, the key, the signature, then canonical spelling.
    pub fn parse(line: &str) -> Result<Self, Refusal> {
        let Ok(Json::Obj(members)) = loops_eval::parse_json(line) else {
            return Err(Refusal::NotJson);
        };
        let opening = names_are(&members, &["env", "id", "payload", "salt", "sig"]);
        if !opening && !names_are(&members, &["commit", "env", "id", "sig"]) {
            return Err(Refusal::Shape);
        }
        let env = member(&members, "env").ok_or(Refusal::Shape)?;
        let env = envelope::read("env", env).map_err(|_| Refusal::Shape)?;
        let stored = if opening {
            let salt = base32::decode(text(&members, "salt")?)
                .ok_or(Refusal::Write(stored::Error::SaltNot16Bytes))?;
            let payload = member(&members, "payload").ok_or(Refusal::Shape)?.clone();
            stored::write(env, salt, payload)
                .map_err(Refusal::Write)?
                .stored()
        } else {
            if let Some(error) = env.problem() {
                return Err(Refusal::Write(error));
            }
            let commit = text(&members, "commit")?;
            if commit
                .strip_prefix("h:")
                .and_then(base32::decode)
                .is_none_or(|bytes| bytes.len() != 32)
            {
                return Err(Refusal::CommitInvalid);
            }
            Stored {
                env,
                body: Body::Erased(commit.to_owned()),
            }
        };
        let id = text(&members, "id")?;
        if stored.id(&hash) != id {
            return Err(Refusal::IdMismatch);
        }
        let Some(Json::Obj(sig)) = member(&members, "sig") else {
            return Err(Refusal::Shape);
        };
        if !names_are(sig, &["ed25519", "key"]) {
            return Err(Refusal::Shape);
        }
        let key = text(sig, "key")?;
        let signature = text(sig, "ed25519")?;
        let point: [u8; 32] = key
            .strip_prefix("k:")
            .and_then(base32::decode)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(Refusal::KeyInvalid)?;
        let verifier = VerifyingKey::from_bytes(&point).map_err(|_| Refusal::KeyInvalid)?;
        let bytes: [u8; 64] = base32::decode(signature)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(Refusal::SignatureInvalid)?;
        verifier
            .verify_strict(&stored::signing_input(id), &Signature::from_bytes(&bytes))
            .map_err(|_| Refusal::SignatureInvalid)?;
        let held = Self {
            id: id.to_owned(),
            stored,
            key: key.to_owned(),
            sig: signature.to_owned(),
        };
        if held.render() == line {
            Ok(held)
        } else {
            Err(Refusal::NotCanonical)
        }
    }

    /// Writes the canonical line, without a line ending.
    pub fn render(&self) -> String {
        self.json().canon()
    }
    /// The line as a JSON value.
    pub fn json(&self) -> Json {
        let mut members = vec![
            ("id".to_owned(), Json::Str(self.id.clone())),
            ("env".to_owned(), self.stored.env.to_json()),
            (
                "sig".to_owned(),
                Json::Obj(vec![
                    ("key".to_owned(), Json::Str(self.key.clone())),
                    ("ed25519".to_owned(), Json::Str(self.sig.clone())),
                ]),
            ),
        ];
        match &self.stored.body {
            Body::Opening { salt, payload } => {
                members.push(("salt".to_owned(), Json::Str(base32::base32(salt))));
                members.push(("payload".to_owned(), payload.clone()));
            }
            Body::Erased(commit) => {
                members.push(("commit".to_owned(), Json::Str(commit.clone())));
            }
        }
        Json::Obj(members)
    }

    /// The record id, as text.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// The envelope.
    pub fn env(&self) -> &Env {
        &self.stored.env
    }
    /// The payload, when this line still holds its opening.
    pub fn payload(&self) -> Option<&Json> {
        match &self.stored.body {
            Body::Opening { payload, .. } => Some(payload),
            Body::Erased(_) => None,
        }
    }
    /// The key that signed the id: `k:` and base32hex.
    pub fn key(&self) -> &str {
        &self.key
    }
    /// The same record with its opening replaced by the commitment.
    /// The id and the signature are unchanged.
    #[must_use]
    pub fn erased(&self) -> Self {
        Self {
            stored: self.stored.erase(&hash),
            ..self.clone()
        }
    }

    /// The record the core's readers take.
    ///
    /// Each top-level text, numeric or boolean member of a payload object
    /// becomes a named field with its written value. Nested values, nulls,
    /// and payloads that are not objects stay in the line, unread by `where`
    /// tests and folds. A line without its opening reads as erased, which
    /// settling shows as withheld unless a held record erases it.
    ///
    /// # Panics
    /// Never for a line made by [`Line::sign`] or [`Line::parse`]: both check
    /// the ids and times this reads.
    pub fn record(&self) -> Record {
        let env = self.env();
        let ids = |xs: &[String]| {
            loops::json::canon_set(xs)
                .iter()
                .map(|s| digest(s))
                .collect()
        };
        let envelope = Record {
            id: digest(&self.id),
            kind: env.kind.clone(),
            observer: env.observer.clone(),
            about: loops::json::canon_set(&env.about),
            of: env.of.clone(),
            prev: ids(&env.prev),
            under: env.under.as_deref().map(digest),
            occurred: env.occurred.as_deref().map(|s| {
                Claim::of(s).expect("a held line's occurred passed the envelope's span check")
            }),
            emitted: Stamp::of(&env.emitted)
                .expect("a held line's emitted passed the envelope's instant check"),
            erases: ids(&env.erases),
            sets: env.sets.as_deref().map(loops::json::canon_set),
            ..Record::default()
        };
        let Some(payload) = self.payload() else {
            return envelope.erase();
        };
        let mut record = envelope;
        if let Json::Obj(members) = payload {
            for (name, value) in members {
                let field = match value {
                    Json::Str(s) => Val::Str(s.clone()),
                    Json::Int(i) => Val::Int(i.clone()),
                    Json::Number(x) => Val::Number(*x),
                    Json::Bool(b) => Val::Bool(*b),
                    _ => continue,
                };
                record.fields.push((name.clone(), field));
            }
        }
        record
    }
}
