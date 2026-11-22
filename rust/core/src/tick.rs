//! The store-independent tick payload codec from model/Loops/Tick.lean.
use crate::{
    base32,
    json::Json,
    read::optional,
    record::{Digest, Record, Shown, id_text, withholding},
    result::{read_result, record_id_of, result_hash_of, result_json},
    spec::{SlotValue, Spec, member_path},
    spec_hash::{Rules, spec_hash_of},
    things::{ids_of, settle},
    where_::{Refusal, RefusalCode},
};
use std::{collections::HashSet, fmt};

/// A tick as interpreted under its evaluation rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickPayload {
    /// The version that gives the spec and result their meaning.
    pub rules: Rules,
    /// The parsed spec.
    pub spec: Spec,
    /// Covered record ids in ascending order.
    pub covers: Vec<Digest>,
    /// The covered ids whose payloads were unavailable, in ascending order.
    pub withheld: Vec<Digest>,
    /// The claimed result in spec slot order.
    pub result: Vec<(String, SlotValue)>,
}

/// The raw members after the hash checks applicable to this reader.
#[derive(Debug, PartialEq, Eq)]
pub struct TickMembers<'a> {
    /// The version named in the hash label.
    rules: Rules,
    /// Whether this reader can interpret the checked members under its own rules.
    own_rules: bool,
    /// The supplied spec object, before parsing.
    spec: &'a Json,
    /// Covered record ids in ascending order.
    covers: Vec<Digest>,
    /// Covered ids whose payloads were unavailable.
    withheld: Vec<Digest>,
    /// The supplied result before checking its slot shapes.
    result: &'a Json,
}

/// A wire fault or the reason a hash-checked claim cannot be interpreted here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TickReadError {
    /// The first wire fault and its path.
    Invalid(Refusal),
    /// The claim names a different rules version.
    OtherRules(Rules),
    /// The spec's vocabulary cannot be parsed under the reader's rules.
    UnreadableSpec,
}
impl fmt::Display for TickReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(fault) => fault.fmt(f),
            Self::OtherRules(rules) => write!(f, "other rules: {}", rules.text()),
            Self::UnreadableSpec => f.write_str("cannot read the spec"),
        }
    }
}
impl std::error::Error for TickReadError {}

/// Lean: `Loops.tickIds`. Encode ids in their supplied order.
pub fn tick_ids(ids: &[Digest]) -> Json {
    Json::Arr(ids.iter().map(|id| Json::Str(id_text(id))).collect())
}

fn payload_json(
    h: &impl Fn(&[u8]) -> Vec<u8>,
    rules: &Rules,
    spec: Json,
    covers: &[Digest],
    withheld: &[Digest],
    result: Json,
) -> Json {
    let spec_hash = spec_hash_of(h, rules, &spec);
    let result_hash = result_hash_of(h, &result);
    let mut fields = vec![
        ("rules".into(), Json::Str(rules.text())),
        ("spec".into(), spec),
        ("spec-hash".into(), Json::Str(spec_hash)),
        ("covers".into(), tick_ids(covers)),
        ("result".into(), result),
        ("result-hash".into(), Json::Str(result_hash)),
    ];
    if !withheld.is_empty() {
        fields.push(("withheld".into(), tick_ids(withheld)));
    }
    Json::Obj(fields)
}

impl TickPayload {
    /// Encode the parsed spec and claimed result, hashing each member.
    pub fn to_json(&self, h: &impl Fn(&[u8]) -> Vec<u8>) -> Json {
        payload_json(
            h,
            &self.rules,
            self.spec.to_json(),
            &self.covers,
            &self.withheld,
            result_json(&self.result),
        )
    }
    /// UTF-8 JCS of the encoded payload.
    pub fn bytes(&self, h: &impl Fn(&[u8]) -> Vec<u8>) -> Vec<u8> {
        self.to_json(h).canon().into_bytes()
    }
}

/// Lean: `Loops.makeTick`. Cover every supplied id and fold the settled, masked forms.
pub fn make_tick(spec: Spec, forms: &[Record]) -> TickPayload {
    let covers = ids_of(forms);
    let mut open: HashSet<_> = forms
        .iter()
        .filter(|r| r.shown == Shown::Here)
        .map(|r| r.id)
        .collect();
    for id in forms.iter().flat_map(|r| &r.erases) {
        open.remove(id);
    }
    let withheld = covers
        .iter()
        .copied()
        .filter(|id| !open.contains(id))
        .collect();
    let masked: Vec<_> = forms
        .iter()
        .map(|r| withholding(|r| !open.contains(&r.id), r))
        .collect();
    TickPayload {
        rules: Rules::current(),
        covers,
        withheld,
        result: spec.run(&settle(&masked)),
        spec,
    }
}

fn fault(code: RefusalCode, path: impl Into<String>) -> Refusal {
    Refusal {
        code,
        path: path.into(),
    }
}

fn read_ids(path: &str, raw: &Json) -> Result<Vec<Digest>, Refusal> {
    let Json::Arr(xs) = raw else {
        return Err(fault(RefusalCode::TickIdsNotArray, path));
    };
    let ids = xs
        .iter()
        .enumerate()
        .map(|(i, x)| {
            record_id_of(x).ok_or_else(|| fault(RefusalCode::TickIdInvalid, format!("{path}[{i}]")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = HashSet::new();
    if ids.iter().any(|id| !seen.insert(id)) {
        return Err(fault(RefusalCode::TickIdDuplicate, path));
    }
    if !ids.is_sorted() {
        return Err(fault(RefusalCode::TickIdsUnsorted, path));
    }
    Ok(ids)
}

fn withheld_ids(covers: &[Digest], raw: Option<&Json>) -> Result<Vec<Digest>, Refusal> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let ids = read_ids("withheld", raw)?;
    if ids.is_empty() {
        return Err(fault(RefusalCode::TickWithheldEmpty, "withheld"));
    }
    if ids.iter().any(|id| covers.binary_search(id).is_err()) {
        return Err(fault(RefusalCode::TickWithheldOutside, "withheld"));
    }
    Ok(ids)
}

fn hash_text<'a>(path: &str, prefix: &str, j: &'a Json) -> Result<&'a str, Refusal> {
    if let Json::Str(s) = j {
        if s.strip_prefix(prefix)
            .and_then(base32::decode)
            .is_some_and(|bytes| bytes.len() == 32)
        {
            return Ok(s);
        }
    }
    Err(fault(RefusalCode::TickHashInvalid, path))
}

const REQUIRED: [&str; 6] = [
    "rules",
    "spec",
    "spec-hash",
    "covers",
    "result",
    "result-hash",
];

// Tick paths include array indices. Visit a name before its value, in input order.
fn repeat_at(value: &Json, path: Option<&str>) -> Option<String> {
    match value {
        Json::Obj(kvs) => {
            let mut seen = HashSet::new();
            for (key, value) in kvs {
                let child = member_path(path, key);
                if !seen.insert(key) {
                    return Some(child);
                }
                if let Some(repeat) = repeat_at(value, Some(&child)) {
                    return Some(repeat);
                }
            }
            None
        }
        Json::Arr(xs) => xs
            .iter()
            .enumerate()
            .find_map(|(i, x)| repeat_at(x, Some(&format!("{}[{i}]", path.unwrap_or_default())))),
        _ => None,
    }
}

/// Check the shared wire format and the spec hash. Check the result hash's fit
/// for the reader's own and older rules, and its spelling for every version.
///
/// # Errors
/// Returns the first payload fault in the order defined by `Loops.readTickMembers`.
pub fn read_tick_members<'a>(
    h: &impl Fn(&[u8]) -> Vec<u8>,
    current: &Rules,
    payload: &'a Json,
) -> Result<TickMembers<'a>, Refusal> {
    let Json::Obj(kvs) = payload else {
        return Err(fault(RefusalCode::TickNotObject, ""));
    };
    if let Some(path) = repeat_at(payload, None) {
        return Err(fault(RefusalCode::TickMemberDuplicate, path));
    }
    if let Some((k, _)) = kvs
        .iter()
        .find(|(k, _)| k != "withheld" && !REQUIRED.contains(&k.as_str()))
    {
        return Err(fault(RefusalCode::TickMemberUnknown, k));
    }
    let field = |k| optional(kvs, k).ok_or_else(|| fault(RefusalCode::TickMemberMissing, k));
    let rules = field("rules")?;
    let spec = field("spec")?;
    let raw_spec_hash = field("spec-hash")?;
    let covers = field("covers")?;
    let result = field("result")?;
    let raw_result_hash = field("result-hash")?;
    let rules =
        Rules::of_json(rules).ok_or_else(|| fault(RefusalCode::TickRulesInvalid, "rules"))?;
    if !matches!(spec, Json::Obj(_)) {
        return Err(fault(RefusalCode::TickSpecNotObject, "spec"));
    }
    let covers = read_ids("covers", covers)?;
    let withheld = withheld_ids(&covers, optional(kvs, "withheld"))?;
    let spec_hash = hash_text("spec-hash", "spec:", raw_spec_hash)?;
    let result_hash = hash_text("result-hash", "res:", raw_result_hash)?;
    if spec_hash != spec_hash_of(h, &rules, spec) {
        return Err(fault(RefusalCode::TickSpecHashMismatch, "spec-hash"));
    }
    if &rules <= current && result_hash != result_hash_of(h, result) {
        return Err(fault(RefusalCode::TickResultHashMismatch, "result-hash"));
    }
    Ok(TickMembers {
        own_rules: &rules == current,
        rules,
        spec,
        covers,
        withheld,
        result,
    })
}

impl TickMembers<'_> {
    /// The verified spec hash's rules version.
    pub fn rules(&self) -> &Rules {
        &self.rules
    }

    /// The verified raw spec object.
    pub fn spec(&self) -> &Json {
        self.spec
    }

    /// The covered record ids.
    pub fn covers(&self) -> &[Digest] {
        &self.covers
    }

    /// The subset of covered ids whose payloads were unavailable.
    pub fn withheld(&self) -> &[Digest] {
        &self.withheld
    }

    /// The raw result member. Its hash has been checked for fit under own or older rules.
    pub fn result(&self) -> &Json {
        self.result
    }

    /// Encode the raw members using this codec's result hash label.
    pub fn to_json(&self, h: &impl Fn(&[u8]) -> Vec<u8>) -> Json {
        payload_json(
            h,
            &self.rules,
            self.spec.clone(),
            &self.covers,
            &self.withheld,
            self.result.clone(),
        )
    }

    /// Interpret the claim under the rules passed to [`read_tick_members`].
    ///
    /// # Errors
    /// Returns the other version, an unreadable spec, or a canonical spec/result fault.
    pub fn read_payload(self) -> Result<TickPayload, TickReadError> {
        if !self.own_rules {
            return Err(TickReadError::OtherRules(self.rules));
        }
        let spec = Spec::of_json(self.spec).map_err(|_| TickReadError::UnreadableSpec)?;
        if self.spec.canon() != spec.to_json().canon() {
            return Err(TickReadError::Invalid(fault(
                RefusalCode::TickSpecNoncanonical,
                "spec",
            )));
        }
        let result = read_result(&spec, self.result)
            .map_err(|path| TickReadError::Invalid(fault(RefusalCode::TickResultInvalid, path)))?;
        Ok(TickPayload {
            rules: self.rules,
            spec,
            covers: self.covers,
            withheld: self.withheld,
            result,
        })
    }
}

/// Read the shared members, then interpret the claim under `current`.
///
/// # Errors
/// Returns the first wire fault or the reason the claim cannot be interpreted.
pub fn read_tick_payload(
    h: &impl Fn(&[u8]) -> Vec<u8>,
    current: &Rules,
    payload: &Json,
) -> Result<TickPayload, TickReadError> {
    read_tick_members(h, current, payload)
        .map_err(TickReadError::Invalid)?
        .read_payload()
}
