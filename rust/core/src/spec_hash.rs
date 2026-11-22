//! Versioned spec identity from model/Loops/SpecHash.lean.
use crate::{base32, json::Json, number::Nat, spec::Spec};

/// A strictly positive evaluation rules version, ordered numerically.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rules(Nat);

impl Rules {
    /// The rules under which specs are currently evaluated.
    pub fn current() -> Self {
        Self(1_u64.into())
    }

    /// Read a JSON string of ASCII decimal digits with a nonzero first digit.
    pub fn of_json(value: &Json) -> Option<Self> {
        let Json::Str(text) = value else { return None };
        let [b'1'..=b'9', rest @ ..] = text.as_bytes() else {
            return None;
        };
        if rest.iter().all(u8::is_ascii_digit) {
            text.parse().ok().map(Self)
        } else {
            None
        }
    }

    /// The canonical decimal spelling used in the hash label.
    pub fn text(&self) -> String {
        self.0.to_string()
    }
}

/// Lean: `Loops.specLabel`. The domain and rules version framing the hash.
pub fn spec_label(rules: &Rules) -> String {
    format!("loops/spec-hash/v{}:", rules.text())
}

/// Lean: `Loops.specPreimageOf`. The label followed immediately by UTF-8 JCS.
pub fn spec_preimage_of(rules: &Rules, member: &Json) -> Vec<u8> {
    let mut bytes = spec_label(rules).into_bytes();
    bytes.extend_from_slice(member.canon().as_bytes());
    bytes
}

/// Lean: `Loops.specHashOf`. Hash a raw member before interpreting its spec language.
pub fn spec_hash_of(h: &impl Fn(&[u8]) -> Vec<u8>, rules: &Rules, member: &Json) -> String {
    format!(
        "spec:{}",
        base32::base32(&h(&spec_preimage_of(rules, member)))
    )
}

impl Spec {
    /// Lean: `Loops.Spec.hash`. Hash the parsed form under the supplied rules.
    pub fn hash(&self, h: &impl Fn(&[u8]) -> Vec<u8>, rules: &Rules) -> String {
        spec_hash_of(h, rules, &self.to_json())
    }
}
