//! Label resolution and batch writer checks from model/Loops/Batch.lean.
use crate::envelope::Env;
use crate::json::Json;
use crate::number::Nat;
use crate::read::{members, optional, required, text};
use crate::stored::{self, Stored, Written};
use num_bigint::BigUint;
use std::fmt;

/// Lean: `Loops.Draft`, model/Loops/Batch.lean.
/// A draft holds an unchecked record and an optional batch label.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Draft {
    /// The optional name that other drafts can use in about, of, prev or under.
    pub label: Option<String>,
    /// The unchecked envelope, which can contain labels in about, of, prev and under.
    pub env: Env,
    /// The bytes that open the payload commitment.
    pub salt: Vec<u8>,
    /// The raw payload that the record commits to.
    pub payload: Json,
}

/// Lean: `Loops.Read.draftValues`, model/Loops/Read.lean.
/// A salt that is not base32hex text reads as no bytes, which the writer
/// refuses as `salt-not-16-bytes`.
fn read_values(path: &str, j: &Json) -> Result<Draft, crate::read::Error> {
    let kvs = members(path, &["label", "env", "salt", "payload"], j)?;
    Ok(Draft {
        label: optional(kvs, "label")
            .map(|v| text(&format!("{path}.label"), v).map(str::to_owned))
            .transpose()?,
        env: crate::envelope::decode_values(
            &format!("{path}.env"),
            optional(kvs, "env").unwrap_or(&Json::Null),
        )?,
        salt: crate::envelope::read_salt(optional(kvs, "salt")),
        payload: required(path, kvs, "payload")?.clone(),
    })
}

/// Lean: `Loops.draftsShapeProblem`, model/Loops/Envelope.lean.
/// Every occurrence of each draft's `env` member; a draft object without one
/// has an envelope that is not an object. A draft that is not an object is
/// left for the value decoder, which refuses it as malformed input.
fn check_draft_shapes<'a>(
    drafts: impl IntoIterator<Item = &'a Json>,
) -> Result<(), crate::read::Error> {
    let envs: Vec<&Json> = drafts
        .into_iter()
        .filter_map(|draft| {
            if let Json::Obj(fields) = draft {
                Some(fields)
            } else {
                None
            }
        })
        .flat_map(|fields| {
            let envs: Vec<&Json> = fields
                .iter()
                .filter_map(|(key, value)| (key == "env").then_some(value))
                .collect();
            if envs.is_empty() {
                vec![&Json::Null]
            } else {
                envs
            }
        })
        .collect();
    crate::envelope::check_all_shapes(envs).map_err(|e| crate::read::Error::from(e.to_string()))
}

/// Checks every raw draft env occurrence, then reads the intervening input,
/// then decodes draft values. The callback preserves held-input order.
///
/// # Errors
/// Returns a raw input error from shape, intervening input or draft decoding.
pub fn read_all<T>(
    path: &str,
    j: &Json,
    before_values: impl FnOnce() -> Result<T, crate::read::Error>,
) -> Result<(T, Vec<Draft>), crate::read::Error> {
    let raw = crate::read::list(path, j)?;
    check_draft_shapes(raw)?;
    let before = before_values()?;
    let drafts = raw
        .iter()
        .enumerate()
        .map(|(i, j)| read_values(&format!("{path}[{i}]"), j))
        .collect::<Result<_, _>>()?;
    Ok((before, drafts))
}

/// Decodes one draft after checking every raw env member.
///
/// # Errors
/// Returns a raw input error for invalid keys, shapes or values.
pub fn read(path: &str, j: &Json) -> Result<Draft, crate::read::Error> {
    check_draft_shapes([j])?;
    read_values(path, j)
}

/// Lean: refusals in `Loops.resolveBatch` and `Loops.checkPrev`, model/Loops/Batch.lean.
/// Each value identifies one batch refusal or a writer refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// More than one draft gives the same label.
    LabelDuplicate,
    /// A draft uses a label that no draft gives.
    LabelUnknown,
    /// The remaining drafts cannot resolve within the supplied fuel.
    LabelCycle,
    /// A resolved draft fails a writer check.
    Write(stored::Error),
    /// A prev target is absent from the batch and held store.
    PrevUnknown,
    /// An under target is absent from the batch and held store.
    UnderUnknown,
    /// An `of` names a held record that is not a root.
    OfNotRoot,
    /// A prev target is of a different thing from the record.
    PrevUnrelated,
}

impl Error {
    /// Lean: refusal strings in `Loops.resolveBatch` and `Loops.checkPrev`, model/Loops/Batch.lean.
    /// Returns the exact wire code, including codes from the writer.
    pub fn code(self) -> &'static str {
        match self {
            Self::LabelDuplicate => "label-duplicate",
            Self::LabelUnknown => "label-unknown",
            Self::LabelCycle => "label-cycle",
            Self::Write(error) => error.code(),
            Self::PrevUnknown => "prev-unknown",
            Self::UnderUnknown => "under-unknown",
            Self::OfNotRoot => "of-not-root",
            Self::PrevUnrelated => "prev-unrelated",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Write(error) => Some(error),
            _ => None,
        }
    }
}

impl From<stored::Error> for Error {
    fn from(error: stored::Error) -> Self {
        Self::Write(error)
    }
}

/// Lean: `Loops.isLabel`, model/Loops/Batch.lean.
/// A leading at sign marks a reference as a label, including the bare at sign.
pub fn is_label(s: &str) -> bool {
    s.starts_with('@')
}

/// Lean: `Loops.subst`, model/Loops/Batch.lean.
/// Replaces labels only, preserves order and repeats, and returns None for a missing label.
pub fn subst(ss: &[String], mapping: impl Fn(&str) -> Option<String>) -> Option<Vec<String>> {
    ss.iter()
        .map(|s| {
            if is_label(s) {
                mapping(s)
            } else {
                Some(s.clone())
            }
        })
        .collect()
}

/// Lean: `Loops.substOpt`, model/Loops/Batch.lean.
/// Replaces a label in an optional field, and returns None for a missing label.
pub fn subst_opt(
    s: Option<&String>,
    mapping: impl Fn(&str) -> Option<String>,
) -> Option<Option<String>> {
    match s {
        None => Some(None),
        Some(s) if is_label(s) => mapping(s).map(Some),
        Some(s) => Some(Some(s.clone())),
    }
}

/// Lean: `Loops.substEnv`, model/Loops/Batch.lean.
/// Replaces labels in about, of, prev and under only; all other envelope
/// fields, `erases` among them, keep their original values.
pub fn subst_env(env: &Env, mapping: impl Fn(&str) -> Option<String>) -> Option<Env> {
    Some(Env {
        about: subst(&env.about, &mapping)?,
        of: subst_opt(env.of.as_ref(), &mapping)?,
        prev: subst(&env.prev, &mapping)?,
        under: subst_opt(env.under.as_ref(), &mapping)?,
        ..env.clone()
    })
}

fn lookup<'a>(mapping: &'a [(String, String)], label: &str) -> Option<&'a str> {
    mapping
        .iter()
        .find(|(key, _)| key == label)
        .map(|(_, value)| value.as_str())
}

impl Draft {
    /// Lean: `Loops.Draft.refs`, model/Loops/Batch.lean.
    /// Lists labels from about, then of, then prev, then under, with their order and repeats unchanged.
    pub fn refs(&self) -> Vec<String> {
        self.env
            .about
            .iter()
            .chain(&self.env.of)
            .chain(&self.env.prev)
            .chain(&self.env.under)
            .filter(|s| is_label(s))
            .cloned()
            .collect()
    }

    /// Lean: `Loops.Draft.commit`, model/Loops/Batch.lean.
    /// Hashes the salt and canonical payload without checking writer rules.
    pub fn commit(&self, h: &impl Fn(&[u8]) -> Vec<u8>) -> String {
        stored::commit_of(h, &self.salt, &self.payload)
    }

    /// Lean: `Loops.Draft.ready`, model/Loops/Batch.lean.
    /// Checks that the mapping contains each label this draft uses.
    pub fn ready(&self, mapping: &[(String, String)]) -> bool {
        self.refs().iter().all(|r| lookup(mapping, r).is_some())
    }
}

/// Lean: `Loops.resolveRounds`, model/Loops/Batch.lean.
/// Resolves each round against its starting mapping; preserves mapping order, repeats, and raw label text.
/// # Errors
/// Returns label-cycle when fuel runs out or no remaining draft is ready.
pub fn resolve_rounds(
    h: &impl Fn(&[u8]) -> Vec<u8>,
    fuel: &Nat,
    mapping: &[(String, String)],
    pending: &[Draft],
) -> Result<Vec<(String, String)>, Error> {
    let mut fuel = fuel.0.clone();
    let mut mapping = mapping.to_vec();
    let mut pending: Vec<_> = pending.iter().collect();
    while !pending.is_empty() {
        if fuel == BigUint::default() {
            return Err(Error::LabelCycle);
        }
        let (ready, waiting): (Vec<_>, Vec<_>) =
            pending.into_iter().partition(|d| d.ready(&mapping));
        if ready.is_empty() {
            return Err(Error::LabelCycle);
        }
        let given: Vec<_> = ready
            .into_iter()
            .filter_map(|d| {
                let label = d.label.as_ref()?;
                let env = subst_env(&d.env, |r| lookup(&mapping, r).map(str::to_owned))?;
                Some((label.clone(), stored::id_of(h, &env, &d.commit(h))))
            })
            .collect();
        mapping.extend(given);
        pending = waiting;
        fuel -= 1_u8;
    }
    Ok(mapping)
}

/// Lean: `Loops.Held`, model/Loops/Batch.lean.
/// What a writer reads of a record it holds: its `of` and its `prev`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Held {
    /// The record's `of`, as written.
    pub of: Option<String>,
    /// The record's `prev`, as written.
    pub prev: Vec<String>,
}

impl Held {
    /// Lean: `Loops.Held.root`, model/Loops/Batch.lean.
    /// A root has no `of` and no `prev`.
    pub fn root(&self) -> bool {
        self.of.is_none() && self.prev.is_empty()
    }
}

/// Lean: `Loops.envThing`, model/Loops/Batch.lean.
/// The thing a record is of, as `Record.thing` reads it: `of`, or its own id.
pub fn env_thing<'a>(id: &'a str, of: Option<&'a str>) -> &'a str {
    of.unwrap_or(id)
}

/// Lean: `Loops.checkPrev`, model/Loops/Batch.lean.
/// Looks a target up in the batch before the held lookup. Checks every
/// unknown prev target first, then every unknown under target, then every
/// `of` that names a held non-root, then every prev target of another thing,
/// each across the whole batch. An under target is only looked up: what it
/// is, a grant or not, is standing's question.
/// # Errors
/// Returns prev-unknown for an absent prev target, then under-unknown for an
/// absent under target, then of-not-root, then prev-unrelated.
pub fn check_prev(
    h: &impl Fn(&[u8]) -> Vec<u8>,
    held: &impl Fn(&str) -> Option<Held>,
    rs: &[Stored],
) -> Result<(), Error> {
    let in_batch: Vec<_> = rs
        .iter()
        .map(|r| {
            let held = Held {
                of: r.env.of.clone(),
                prev: r.env.prev.clone(),
            };
            (r.id(h), held)
        })
        .collect();
    let target = |p: &str| {
        in_batch
            .iter()
            .find(|(id, _)| id == p)
            .map(|(_, held)| held.clone())
            .or_else(|| held(p))
    };
    if rs
        .iter()
        .any(|r| r.env.prev.iter().any(|p| target(p).is_none()))
    {
        return Err(Error::PrevUnknown);
    }
    if rs
        .iter()
        .any(|r| r.env.under.iter().any(|u| target(u).is_none()))
    {
        return Err(Error::UnderUnknown);
    }
    if rs.iter().any(|r| {
        r.env
            .of
            .iter()
            .any(|o| target(o).is_some_and(|t| !t.root()))
    }) {
        return Err(Error::OfNotRoot);
    }
    if rs.iter().any(|r| {
        let id = r.id(h);
        let thing = env_thing(&id, r.env.of.as_deref());
        r.env
            .prev
            .iter()
            .any(|p| target(p).is_some_and(|t| env_thing(p, t.of.as_deref()) != thing))
    }) {
        return Err(Error::PrevUnrelated);
    }
    Ok(())
}

// Lean: Loops.writeCodes. The order is explicit and independent of enum layout.
fn writer_priority(error: stored::Error) -> u8 {
    match error {
        stored::Error::KindEmpty => 1,
        stored::Error::ObserverNotTyped => 2,
        stored::Error::EmittedNotRfc3339 => 3,
        stored::Error::OccurredNotSpan => 4,
        stored::Error::AboutNotTyped => 5,
        stored::Error::OfNotTyped => 6,
        stored::Error::PrevNotRecordId => 7,
        stored::Error::UnderNotRecordId => 8,
        stored::Error::ErasesNotRecordId => 9,
        stored::Error::OfMissing => 10,
        stored::Error::SaltNot16Bytes => 11,
        stored::Error::MemberDuplicate => 12,
        stored::Error::NumberOutOfRange => 13,
    }
}

/// Lean: `Loops.resolveBatch`, model/Loops/Batch.lean.
/// Resolves labels, writes checked records, and checks prev targets while preserving the original draft order.
/// # Errors
/// Checks duplicate labels, unknown labels, cycles, global writer priority, unknown prev targets,
/// unknown under targets, an `of` naming a held non-root, then prev targets of another thing.
pub fn resolve_batch(
    h: &impl Fn(&[u8]) -> Vec<u8>,
    held: &impl Fn(&str) -> Option<Held>,
    ds: &[Draft],
) -> Result<Vec<Written>, Error> {
    let labels: Vec<_> = ds.iter().filter_map(|d| d.label.as_deref()).collect();
    if labels
        .iter()
        .enumerate()
        .any(|(i, label)| labels[..i].contains(label))
    {
        return Err(Error::LabelDuplicate);
    }
    if !ds
        .iter()
        .all(|d| d.refs().iter().all(|r| labels.contains(&r.as_str())))
    {
        return Err(Error::LabelUnknown);
    }
    let mapping = resolve_rounds(h, &Nat(BigUint::from(ds.len())), &[], ds)?;
    let resolved: Vec<_> = ds
        .iter()
        .map(|d| {
            subst_env(&d.env, |r| lookup(&mapping, r).map(str::to_owned))
                .map(|env| (env, d))
                .ok_or(Error::LabelCycle)
        })
        .collect::<Result<_, _>>()?;
    if let Some(error) = resolved
        .iter()
        .filter_map(|(env, d)| stored::write_problem(env, &d.salt, &d.payload))
        .min_by_key(|error| writer_priority(*error))
    {
        return Err(Error::Write(error));
    }
    let written: Vec<_> = resolved
        .into_iter()
        .map(|(env, d)| stored::write(env, d.salt.clone(), d.payload.clone()).map_err(Error::from))
        .collect::<Result<_, _>>()?;
    let records: Vec<_> = written.iter().map(Written::stored).collect();
    check_prev(h, held, &records)?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::number::Int;
    use sha2::{Digest, Sha256};
    use std::cell::Cell;

    #[test]
    fn raw_draft_shape_precedes_labels_and_intervening_input() {
        let unknown = Json::Obj(vec![("extra".into(), Json::Null)]);
        // A malformed label beside an envelope that passes shape.
        let malformed = Json::Obj(vec![
            ("label".into(), Json::Null),
            ("env".into(), Json::Obj(vec![])),
        ]);
        // Both env occurrences must survive, in either order.
        for envs in [[unknown.clone(), Json::Null], [Json::Null, unknown]] {
            let repeated = Json::Obj(vec![
                ("env".into(), envs[0].clone()),
                ("env".into(), envs[1].clone()),
                ("label".into(), Json::Null),
            ]);
            for drafts in [
                vec![malformed.clone(), repeated.clone()],
                vec![repeated, malformed.clone()],
            ] {
                let called = Cell::new(false);
                let result = read_all("drafts", &Json::Arr(drafts), || {
                    called.set(true);
                    Err::<(), _>(crate::read::Error::from("held values".to_owned()))
                });
                assert_eq!(
                    result.expect_err("shape").to_string(),
                    "envelope-field-unknown"
                );
                assert!(!called.get());
            }
        }
        let result = read_all("drafts", &Json::Arr(vec![malformed.clone()]), || {
            Err::<(), _>(crate::read::Error::from("held values".to_owned()))
        });
        assert_eq!(
            result.expect_err("held before draft values").to_string(),
            "held values"
        );
        // A draft without an envelope, or with one that is not an object, is a
        // shape refusal, before held input and draft values; an unknown key wins.
        let no_env = Json::Obj(vec![("label".into(), Json::Null)]);
        let not_object = Json::Obj(vec![("env".into(), Json::Int(7_i64.into()))]);
        for drafts in [
            vec![no_env.clone()],
            vec![malformed.clone(), not_object.clone()],
            vec![not_object, malformed],
        ] {
            let called = Cell::new(false);
            let result = read_all("drafts", &Json::Arr(drafts), || {
                called.set(true);
                Ok(())
            });
            assert_eq!(
                result.expect_err("shape").to_string(),
                "envelope-not-object"
            );
            assert!(!called.get());
        }
        let with_unknown = Json::Obj(vec![(
            "env".into(),
            Json::Obj(vec![("extra".into(), Json::Null)]),
        )]);
        assert_eq!(
            read_all("drafts", &Json::Arr(vec![no_env, with_unknown]), || Ok(()))
                .expect_err("shape")
                .to_string(),
            "envelope-field-unknown"
        );
    }

    fn hash(bytes: &[u8]) -> Vec<u8> {
        Sha256::digest(bytes).to_vec()
    }

    fn no_held(_: &str) -> Option<Held> {
        None
    }

    fn draft(label: Option<&str>) -> Draft {
        Draft {
            label: label.map(str::to_owned),
            env: Env {
                kind: "note".into(),
                observer: "person:alice".into(),
                emitted: "2025-01-01T00:00:00Z".into(),
                ..Env::default()
            },
            salt: vec![0; 16],
            payload: Json::Null,
        }
    }

    fn texts(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).to_owned()).collect()
    }

    fn duplicate_payload() -> Json {
        Json::Arr(vec![Json::Obj(vec![
            ("same".into(), Json::Null),
            ("same".into(), Json::Bool(true)),
        ])])
    }

    fn big_payload() -> Json {
        Json::Int(Int::from(9_007_199_254_740_992_u64))
    }

    #[test]
    fn substitution_keeps_order_duplicates_and_unrelated_fields() {
        assert!(is_label("@"));
        assert!(is_label("@thing"));
        assert!(!is_label("thing@"));
        assert!(!is_label(""));
        let input = texts(&["x:one", "@a", "@a"]);
        let mapping = |r: &str| (r == "@a").then(|| "r:resolved".into());
        assert_eq!(
            subst(&input, mapping),
            Some(texts(&["x:one", "r:resolved", "r:resolved"]))
        );
        assert_eq!(subst(&texts(&["@missing"]), mapping), None);
        assert_eq!(subst(&[], mapping), Some(vec![]));
        assert_eq!(subst_opt(None, mapping), Some(None));
        assert_eq!(
            subst_opt(Some(&"x:one".into()), mapping),
            Some(Some("x:one".into()))
        );
        assert_eq!(
            subst_opt(Some(&"@a".into()), mapping),
            Some(Some("r:resolved".into()))
        );
        assert_eq!(subst_opt(Some(&"@missing".into()), mapping), None);
        let mut d = draft(Some("@own"));
        d.env.about = input;
        d.env.of = Some("@a".into());
        d.env.prev = texts(&["@a"]);
        d.env.kind = "@kind".into();
        d.env.observer = "@observer".into();
        d.env.emitted = "@emitted".into();
        d.env.occurred = Some("@occurred".into());
        d.env.under = Some("@a".into());
        d.env.erases = texts(&["@erases"]);
        d.payload = Json::Str("@payload".into());
        assert_eq!(d.refs(), texts(&["@a", "@a", "@a", "@a", "@a"]));
        let mut expected = d.env.clone();
        expected.about = texts(&["x:one", "r:resolved", "r:resolved"]);
        expected.of = Some("r:resolved".into());
        expected.prev = texts(&["r:resolved"]);
        expected.under = Some("r:resolved".into());
        assert_eq!(subst_env(&d.env, mapping), Some(expected));
        d.env.of = Some("@missing".into());
        assert_eq!(subst_env(&d.env, mapping), None);
        d.env.of = Some("@a".into());
        d.env.under = Some("@missing".into());
        assert_eq!(subst_env(&d.env, mapping), None);
        d.env.under = Some("@a".into());
        assert_eq!(
            d.commit(&hash),
            stored::commit_of(&hash, &d.salt, &d.payload)
        );
        d.env.prev = texts(&["@missing"]);
        assert_eq!(subst_env(&d.env, mapping), None);
    }

    #[test]
    fn rounds_use_old_mapping_and_first_lookup_without_deduplication() {
        let seed = vec![
            ("@seed".into(), "thing:first".into()),
            ("@seed".into(), "thing:second".into()),
        ];
        let mut a = draft(Some("@a"));
        a.env.about = texts(&["@seed"]);
        let mut b = draft(Some("@b"));
        b.env.about = texts(&["@a"]);
        let pending = [a.clone(), b.clone()];
        assert!(a.ready(&seed));
        assert!(!b.ready(&seed));
        assert_eq!(
            resolve_rounds(&hash, &Nat::from(1), &seed, &pending),
            Err(Error::LabelCycle)
        );
        let got =
            resolve_rounds(&hash, &Nat::from(2), &seed, &pending).expect("two dependency rounds");
        let mut env_a = a.env.clone();
        env_a.about = texts(&["thing:first"]);
        let id_a = stored::id_of(&hash, &env_a, &a.commit(&hash));
        let mut env_b = b.env.clone();
        env_b.about = vec![id_a.clone()];
        let id_b = stored::id_of(&hash, &env_b, &b.commit(&hash));
        assert_eq!(
            got,
            [seed.clone(), vec![("@a".into(), id_a), ("@b".into(), id_b)]].concat()
        );

        let mut second_a = a.clone();
        second_a.payload = Json::Bool(true);
        let duplicates =
            resolve_rounds(&hash, &Nat::from(1), &seed, &[a.clone(), second_a.clone()])
                .expect("raw duplicate labels stay valid");
        assert_eq!(&duplicates[..2], &seed);
        assert_eq!(
            duplicates[2],
            ("@a".into(), stored::id_of(&hash, &env_a, &a.commit(&hash)))
        );
        assert_eq!(
            duplicates[3],
            (
                "@a".into(),
                stored::id_of(&hash, &env_a, &second_a.commit(&hash))
            )
        );

        let mut preexisting = seed;
        preexisting.push(("@a".into(), "thing:old".into()));
        let got = resolve_rounds(&hash, &Nat::from(1), &preexisting, &[a, b.clone()])
            .expect("both drafts use the initial mapping");
        env_b.about = texts(&["thing:old"]);
        assert_eq!(
            got.last(),
            Some(&(
                String::from("@b"),
                stored::id_of(&hash, &env_b, &b.commit(&hash))
            ))
        );
    }

    #[test]
    fn rounds_keep_unbounded_fuel_and_remove_unlabeled_drafts() {
        let mapping = vec![("x".into(), "first".into()), ("x".into(), "second".into())];
        assert_eq!(
            resolve_rounds(&hash, &Nat::default(), &mapping, &[]),
            Ok(mapping.clone())
        );
        assert_eq!(
            resolve_rounds(&hash, &Nat::default(), &mapping, &[draft(None)]),
            Err(Error::LabelCycle)
        );
        assert_eq!(
            resolve_rounds(&hash, &Nat::from(1), &mapping, &[draft(None)]),
            Ok(mapping.clone())
        );
        let fuel: Nat = "1844674407370955161600000000000000000000000000000000000000"
            .parse()
            .expect("unbounded natural");
        assert_eq!(
            resolve_rounds(&hash, &fuel, &mapping, &[draft(None)]),
            Ok(mapping)
        );
        let mut blocked = draft(None);
        blocked.env.about = texts(&["@missing"]);
        assert_eq!(
            resolve_rounds(&hash, &fuel, &[], &[blocked]),
            Err(Error::LabelCycle)
        );
    }

    #[test]
    fn raw_label_text_has_no_extra_validation() {
        let ds = [draft(Some("")), draft(Some("plain")), draft(Some("@"))];
        let mapping = resolve_rounds(&hash, &Nat::from(1), &[], &ds).expect("labels are raw text");
        assert_eq!(
            mapping.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>(),
            ["", "plain", "@"]
        );
        assert_eq!(
            resolve_batch(&hash, &no_held, &ds)
                .expect("unused raw labels are allowed")
                .len(),
            3
        );
        assert_eq!(
            resolve_batch(&hash, &no_held, &[draft(Some("")), draft(Some(""))]),
            Err(Error::LabelDuplicate)
        );
    }

    #[test]
    fn label_refusals_precede_writer_and_prev_refusals() {
        let mut bad = draft(Some("@same"));
        bad.env.kind.clear();
        bad.env.about = texts(&["@unknown"]);
        assert_eq!(
            resolve_batch(&hash, &no_held, &[bad.clone(), draft(Some("@same"))]),
            Err(Error::LabelDuplicate)
        );
        assert_eq!(
            resolve_batch(&hash, &no_held, &[bad.clone()]),
            Err(Error::LabelUnknown)
        );
        bad.env.about = texts(&["@same"]);
        assert_eq!(
            resolve_batch(&hash, &no_held, &[bad]),
            Err(Error::LabelCycle)
        );
        let mut a = draft(Some("@a"));
        let mut b = draft(Some("@b"));
        a.env.about = texts(&["@b"]);
        b.env.prev = texts(&["@a"]);
        assert_eq!(
            resolve_batch(&hash, &no_held, &[a, b]),
            Err(Error::LabelCycle)
        );
        let mut d = draft(None);
        d.env.kind = "@not-a-reference".into();
        d.payload = Json::Str("@not-a-reference".into());
        assert!(resolve_batch(&hash, &no_held, &[d.clone()]).is_ok());
        // A label may stand in `under`: one no draft gives is unknown, one
        // reaching back to its own draft is a cycle, and one given resolves.
        d.env.under = Some("@not-a-reference".into());
        assert_eq!(
            resolve_batch(&hash, &no_held, &[d.clone()]),
            Err(Error::LabelUnknown)
        );
        let mut grant = draft(Some("@g"));
        grant.env.under = Some("@g".into());
        assert_eq!(
            resolve_batch(&hash, &no_held, &[grant.clone()]),
            Err(Error::LabelCycle)
        );
        grant.env.under = None;
        d.env.under = Some("@g".into());
        let written = resolve_batch(&hash, &no_held, &[d.clone(), grant]).expect("under label");
        assert_eq!(
            written[0].stored().env.under,
            Some(written[1].stored().id(&hash))
        );
        d.env.under = None;
        d.env.erases = texts(&["@not-a-reference"]);
        assert_eq!(
            resolve_batch(&hash, &no_held, &[d]),
            Err(Error::Write(stored::Error::ErasesNotRecordId))
        );
    }

    fn invalid_drafts() -> Vec<(Draft, stored::Error, &'static str)> {
        let mut ds = vec![draft(None); 13];
        ds[0].env.kind.clear();
        ds[1].env.observer = "bad".into();
        ds[2].env.emitted = "bad".into();
        ds[3].env.occurred = Some("bad".into());
        ds[4].env.about = texts(&["bad"]);
        ds[5].env.of = Some("bad".into());
        ds[6].env.prev = texts(&["bad"]);
        ds[7].env.under = Some("bad".into());
        ds[8].env.erases = texts(&["bad"]);
        ds[9].env.prev = vec![format!("r:{}", crate::base32::base32(&[0; 32]))];
        ds[10].salt.clear();
        ds[11].payload = duplicate_payload();
        ds[12].payload = big_payload();
        let errors = [
            (stored::Error::KindEmpty, "kind-empty"),
            (stored::Error::ObserverNotTyped, "observer-not-typed"),
            (stored::Error::EmittedNotRfc3339, "emitted-not-rfc3339"),
            (stored::Error::OccurredNotSpan, "occurred-not-span"),
            (stored::Error::AboutNotTyped, "about-not-typed"),
            (stored::Error::OfNotTyped, "of-not-typed"),
            (stored::Error::PrevNotRecordId, "prev-not-record-id"),
            (stored::Error::UnderNotRecordId, "under-not-record-id"),
            (stored::Error::ErasesNotRecordId, "erases-not-record-id"),
            (stored::Error::OfMissing, "of-missing"),
            (stored::Error::SaltNot16Bytes, "salt-not-16-bytes"),
            (stored::Error::MemberDuplicate, "member-duplicate"),
            (stored::Error::NumberOutOfRange, "number-out-of-range"),
        ];
        ds.into_iter()
            .zip(errors)
            .map(|(d, (e, code))| (d, e, code))
            .collect()
    }

    #[test]
    fn every_writer_priority_is_global_in_both_draft_orders() {
        let invalid = invalid_drafts();
        for (index, (earlier, error, code)) in invalid.iter().enumerate() {
            assert_eq!(
                resolve_batch(&hash, &no_held, std::slice::from_ref(earlier)),
                Err(Error::Write(*error)),
                "{code}"
            );
            assert_eq!(Error::Write(*error).code(), *code);
            for (later, _, _) in &invalid[index + 1..] {
                for ds in [
                    [earlier.clone(), later.clone()],
                    [later.clone(), earlier.clone()],
                ] {
                    assert_eq!(
                        resolve_batch(&hash, &no_held, &ds),
                        Err(Error::Write(*error)),
                        "{code}"
                    );
                }
            }
            let mut unknown_prev = draft(None);
            unknown_prev.env.of = Some("thing:x".into());
            unknown_prev.env.prev = vec![format!("r:{}", crate::base32::base32(&[0; 32]))];
            assert_eq!(
                resolve_batch(&hash, &no_held, &[unknown_prev, earlier.clone()]),
                Err(Error::Write(*error)),
                "writer before prev: {code}"
            );
        }
        let mut both = draft(None);
        both.payload = Json::Obj(vec![("a".into(), big_payload()), ("a".into(), Json::Null)]);
        assert_eq!(
            resolve_batch(&hash, &no_held, &[both]),
            Err(Error::Write(stored::Error::MemberDuplicate))
        );
    }

    #[test]
    fn unknown_prev_precedes_unrelated_in_both_orders() {
        let unrelated = stored::make(
            Env {
                of: Some("thing:one".into()),
                ..Env::default()
            },
            vec![],
            Json::Null,
        );
        let target = unrelated.id(&hash);
        let missing = "r:missing".to_owned();
        let held = |id: &str| {
            (id == target).then(|| Held {
                of: Some("thing:one".into()),
                prev: vec![],
            })
        };
        for prev in [
            vec![target.clone(), missing.clone()],
            vec![missing.clone(), target.clone()],
        ] {
            let mut d = draft(None);
            d.env.of = Some("thing:other".into());
            d.env.prev = prev;
            assert_eq!(
                check_prev(&hash, &held, &[stored::make(d.env, d.salt, d.payload)]),
                Err(Error::PrevUnknown)
            );
        }
        let mut unknown = draft(None);
        unknown.env.prev = vec![missing];
        let mut wrong = draft(None);
        wrong.env.of = Some("thing:other".into());
        wrong.env.prev = vec![target.clone()];
        for ds in [[unknown.clone(), wrong.clone()], [wrong.clone(), unknown]] {
            let rs: Vec<_> = ds
                .into_iter()
                .map(|d| stored::make(d.env, d.salt, d.payload))
                .collect();
            assert_eq!(check_prev(&hash, &held, &rs), Err(Error::PrevUnknown));
        }
        assert_eq!(
            check_prev(
                &hash,
                &held,
                &[stored::make(wrong.env, wrong.salt, wrong.payload)]
            ),
            Err(Error::PrevUnrelated)
        );
    }

    #[test]
    fn held_lookup_repeats_and_batch_first_match_overrides_held() {
        let calls = Cell::new(0);
        let held = |p: &str| {
            calls.set(calls.get() + 1);
            (p == "r:external").then(|| Held {
                of: Some("thing:shared".into()),
                prev: vec![],
            })
        };
        let mut d = draft(None);
        d.env.of = Some("thing:shared".into());
        d.env.prev = texts(&["r:external", "r:external"]);
        let r = stored::make(d.env, d.salt, d.payload);
        assert_eq!(check_prev(&hash, &held, &[r]), Ok(()));
        assert_eq!(calls.get(), 5, "three passes keep repeated lookup calls");

        let constant = |_: &[u8]| vec![0];
        let mut first = draft(None);
        first.env.of = Some("thing:first".into());
        let mut second = first.clone();
        second.env.of = Some("thing:second".into());
        let mut successor = first.clone();
        successor.env.prev = vec![stored::id_of(
            &constant,
            &first.env,
            &first.commit(&constant),
        )];
        let rs: Vec<_> = [first, second, successor]
            .into_iter()
            .map(|d| stored::make(d.env, d.salt, d.payload))
            .collect();
        // Only a natural key in `of` reaches the held lookup here.
        let forbidden_held = |p: &str| -> Option<Held> {
            assert!(!p.starts_with("r:"), "batch target must override held");
            None
        };
        assert_eq!(check_prev(&constant, &forbidden_held, &rs), Ok(()));
        let reversed = [rs[1].clone(), rs[0].clone(), rs[2].clone()];
        assert_eq!(
            check_prev(&constant, &forbidden_held, &reversed),
            Err(Error::PrevUnrelated)
        );
    }

    #[test]
    fn a_record_without_of_is_its_own_thing_for_source_and_target() {
        assert_eq!(env_thing("r:self", None), "r:self");
        assert_eq!(env_thing("r:self", Some("t:x")), "t:x");
        assert!(Held::default().root());
        assert!(
            !Held {
                of: Some("t:x".into()),
                prev: vec![]
            }
            .root()
        );
        assert!(
            !Held {
                of: None,
                prev: texts(&["r:p"])
            }
            .root()
        );
        let root = draft(None);
        let id = stored::id_of(&hash, &root.env, &root.commit(&hash));
        let held = |p: &str| (p == id).then(Held::default);
        let mut child = draft(None);
        child.env.of = Some(id.clone());
        child.env.prev = vec![id.clone()];
        assert!(resolve_batch(&hash, &held, &[child.clone()]).is_ok());
        // `about` names the thing but does not make the record of it.
        child.env.about = vec![id.clone()];
        child.env.of = Some("thing:other".into());
        assert_eq!(
            resolve_batch(&hash, &held, &[child.clone()]),
            Err(Error::PrevUnrelated)
        );
        child.env.of = None;
        assert_eq!(
            resolve_batch(&hash, &held, &[child.clone()]),
            Err(Error::Write(stored::Error::OfMissing))
        );
        // An `of` naming a held record that is not a root.
        let revision = |p: &str| {
            (p == id).then(|| Held {
                of: Some("thing:other".into()),
                prev: texts(&["r:p"]),
            })
        };
        child.env.of = Some(id.clone());
        assert_eq!(
            resolve_batch(&hash, &revision, &[child]),
            Err(Error::OfNotRoot)
        );
        // `of` is read as written: a natural key nobody holds is no refusal.
        let mut keyed = draft(None);
        keyed.env.of = Some("thing:anything".into());
        assert!(resolve_batch(&hash, &no_held, &[keyed]).is_ok());
        let constant = |_: &[u8]| vec![0];
        let mut self_prev = root;
        self_prev.env.prev = vec![stored::id_of(
            &constant,
            &self_prev.env,
            &self_prev.commit(&constant),
        )];
        assert_eq!(
            check_prev(
                &constant,
                &no_held,
                &[stored::make(
                    self_prev.env,
                    self_prev.salt,
                    self_prev.payload
                )]
            ),
            Ok(())
        );
    }

    #[test]
    fn label_chain_matches_manual_writes_and_keeps_input_order() {
        let root = draft(Some("@root"));
        let root_written = stored::write(root.env.clone(), root.salt.clone(), root.payload.clone())
            .expect("valid root");
        let root_id = root_written.stored().id(&hash);
        let mut child = draft(Some("@child"));
        child.env.about = texts(&["@root"]);
        child.env.of = Some("@root".into());
        child.env.prev = texts(&["@root"]);
        child.payload = Json::Str("child".into());
        let mut child_env = child.env.clone();
        child_env.about = vec![root_id.clone()];
        child_env.of = Some(root_id.clone());
        child_env.prev = vec![root_id.clone()];
        let child_written = stored::write(child_env, child.salt.clone(), child.payload.clone())
            .expect("valid child");
        let mut leaf = draft(None);
        leaf.env.of = Some("@root".into());
        leaf.env.prev = texts(&["@child"]);
        let mut leaf_env = leaf.env.clone();
        leaf_env.of = Some(root_id);
        leaf_env.prev = vec![child_written.stored().id(&hash)];
        let leaf_written =
            stored::write(leaf_env, leaf.salt.clone(), leaf.payload.clone()).expect("valid leaf");
        assert_eq!(
            resolve_batch(
                &hash,
                &no_held,
                &[leaf.clone(), root.clone(), child.clone()]
            ),
            Ok(vec![
                leaf_written.clone(),
                root_written.clone(),
                child_written.clone()
            ])
        );
        assert_eq!(
            resolve_batch(&hash, &no_held, &[child, leaf, root]),
            Ok(vec![child_written, leaf_written, root_written])
        );
        assert_eq!(resolve_batch(&hash, &no_held, &[]), Ok(vec![]));
    }

    #[test]
    fn batch_prev_refusals_are_global_after_writer_checks() {
        let held_id = format!("r:{}", crate::base32::base32(&[0; 32]));
        let missing_id = format!("r:{}", crate::base32::base32(&[1; 32]));
        let held = |p: &str| {
            (p == held_id).then(|| Held {
                of: Some("thing:held".into()),
                prev: vec![],
            })
        };
        let mut unrelated = draft(None);
        unrelated.env.of = Some("thing:other".into());
        unrelated.env.prev = vec![held_id.clone()];
        let mut unknown = draft(None);
        unknown.env.of = Some("thing:other".into());
        unknown.env.prev = vec![missing_id.clone()];
        let mut not_root = draft(None);
        not_root.env.of = Some(held_id.clone());
        for ds in [
            [unrelated.clone(), unknown.clone()],
            [unknown.clone(), unrelated.clone()],
            [not_root.clone(), unknown.clone()],
            [unknown.clone(), not_root.clone()],
        ] {
            assert_eq!(resolve_batch(&hash, &held, &ds), Err(Error::PrevUnknown));
        }
        for ds in [
            [unrelated.clone(), not_root.clone()],
            [not_root, unrelated.clone()],
        ] {
            assert_eq!(resolve_batch(&hash, &held, &ds), Err(Error::OfNotRoot));
        }
        for prev in [
            vec![held_id.clone(), missing_id.clone()],
            vec![missing_id, held_id.clone()],
        ] {
            let mut d = unrelated.clone();
            d.env.prev = prev;
            assert_eq!(resolve_batch(&hash, &held, &[d]), Err(Error::PrevUnknown));
        }
        assert_eq!(
            resolve_batch(&hash, &held, &[unrelated]),
            Err(Error::PrevUnrelated)
        );
        assert_eq!(
            resolve_batch(&hash, &held, &[unknown]),
            Err(Error::PrevUnknown)
        );
    }

    #[test]
    fn an_under_target_is_held_after_prev_unknown_and_before_the_rest() {
        let held_id = format!("r:{}", crate::base32::base32(&[0; 32]));
        let missing_id = format!("r:{}", crate::base32::base32(&[1; 32]));
        // A held revision: not a root and no grant, but held, which is all `under` asks.
        let held = |p: &str| {
            (p == held_id).then(|| Held {
                of: Some("thing:held".into()),
                prev: texts(&["r:p"]),
            })
        };
        let mut under_held = draft(None);
        under_held.env.under = Some(held_id.clone());
        assert!(resolve_batch(&hash, &held, std::slice::from_ref(&under_held)).is_ok());
        let mut under_missing = draft(None);
        under_missing.env.under = Some(missing_id.clone());
        assert_eq!(
            resolve_batch(&hash, &held, std::slice::from_ref(&under_missing)),
            Err(Error::UnderUnknown)
        );
        // The raw id of a batch member is held, in either order.
        let member = draft(Some("@m"));
        let mut under_member = draft(None);
        under_member.env.under = Some(stored::id_of(&hash, &member.env, &member.commit(&hash)));
        under_member.payload = Json::Str("under a member".into());
        for ds in [
            [member.clone(), under_member.clone()],
            [under_member, member],
        ] {
            assert!(resolve_batch(&hash, &no_held, &ds).is_ok());
        }
        let mut unknown_prev = draft(None);
        unknown_prev.env.of = Some("thing:other".into());
        unknown_prev.env.prev = vec![missing_id.clone()];
        let mut not_root = draft(None);
        not_root.env.of = Some(held_id.clone());
        not_root.payload = Json::Str("not a root".into());
        let mut unrelated = draft(None);
        unrelated.env.of = Some("thing:other".into());
        unrelated.env.prev = vec![held_id.clone()];
        unrelated.payload = Json::Str("unrelated".into());
        for ds in [
            [under_missing.clone(), unknown_prev.clone()],
            [unknown_prev, under_missing.clone()],
        ] {
            assert_eq!(resolve_batch(&hash, &held, &ds), Err(Error::PrevUnknown));
        }
        for other in [not_root, unrelated] {
            for ds in [
                [under_missing.clone(), other.clone()],
                [other, under_missing.clone()],
            ] {
                assert_eq!(resolve_batch(&hash, &held, &ds), Err(Error::UnderUnknown));
            }
        }
        // One draft with both: prev-unknown.
        let mut both = under_missing;
        both.env.of = Some(missing_id.clone());
        both.env.prev = vec![missing_id];
        assert_eq!(
            resolve_batch(&hash, &held, &[both]),
            Err(Error::PrevUnknown)
        );
    }

    #[test]
    fn batch_lookup_finds_a_root_and_hash_has_no_size_constraint() {
        let constant = |_: &[u8]| vec![];
        let root = draft(None);
        let written = resolve_batch(&constant, &no_held, std::slice::from_ref(&root))
            .expect("the hash output has no size constraint");
        assert_eq!(written[0].stored().id(&constant), "r:");
        let mut successor = draft(None);
        successor.env.of = Some("r:".into());
        successor.env.prev = texts(&["r:"]);
        let forbidden_held =
            |_: &str| -> Option<Held> { panic!("a root in the batch still overrides held") };
        let rs: Vec<_> = [root, successor]
            .into_iter()
            .map(|d| stored::make(d.env, d.salt, d.payload))
            .collect();
        assert_eq!(check_prev(&constant, &forbidden_held, &rs), Ok(()));
    }

    #[test]
    fn error_codes_display_and_sources_are_exact() {
        use std::error::Error as _;
        for (error, code) in [
            (Error::LabelDuplicate, "label-duplicate"),
            (Error::LabelUnknown, "label-unknown"),
            (Error::LabelCycle, "label-cycle"),
            (Error::PrevUnknown, "prev-unknown"),
            (Error::UnderUnknown, "under-unknown"),
            (Error::OfNotRoot, "of-not-root"),
            (Error::PrevUnrelated, "prev-unrelated"),
        ] {
            assert_eq!(error.code(), code);
            assert_eq!(error.to_string(), code);
            assert!(error.source().is_none());
        }
        let error = Error::from(stored::Error::MemberDuplicate);
        assert_eq!(error.to_string(), "member-duplicate");
        assert_eq!(
            error.source().map(ToString::to_string),
            Some("member-duplicate".into())
        );
    }
}
