//! Lean: `Loops.Result`, model/Loops/Result.lean: the canonical result (D7).
//!
//! A vertex's whole result, every slot, as canonical bytes and a hash. Every
//! integer is a JSON string of its decimal digits, never a number, so a sum
//! past 2^53 keeps every digit. A listed record is its id alone: `shown` is
//! not written. The bytes are JCS of that JSON in UTF-8; the hash covers the
//! label `loops/result-hash/v1:` and then the bytes. The label is 21 bytes,
//! not 16, so no record's commitment, a 16-byte salt and then JCS, has a
//! result hash's preimage. See `protocol/result.md`.
use crate::{
    base32,
    json::Json,
    number::{Int, Nat},
    record::{Digest, Shown, id_text},
    spec::{GroupValue, KeyedView, SlotValue},
};

/// Lean: `Loops.resultLabel`. The codec's version, which frames what is hashed.
pub const RESULT_LABEL: &str = "loops/result-hash/v1:";

/// Lean: `Loops.intText`. An integer as decimal digits, `-` before a negative.
pub fn int_text(i: &Int) -> String {
    i.to_string()
}

/// Lean: `Loops.intOf?`. An integer as the codec reads it: `0`, or digits
/// with no leading zero, after `-` for a negative. Nothing else is read.
pub fn int_of(text: &str) -> Option<Int> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let canonical = match digits.as_bytes() {
        [b'0'] => digits.len() == text.len(),
        [first, rest @ ..] => {
            *first != b'0' && first.is_ascii_digit() && rest.iter().all(u8::is_ascii_digit)
        }
        [] => false,
    };
    if canonical { text.parse().ok() } else { None }
}

fn int_json(i: &Int) -> Json {
    Json::Str(int_text(i))
}
fn nat_json(n: &Nat) -> Json {
    Json::Str(n.to_string())
}
fn obj(kvs: Vec<(&str, Json)>) -> Json {
    Json::Obj(kvs.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

/// Lean: `Loops.idsJson`. Listed records by id alone, in the order listed.
fn ids_json(hs: &[(Digest, Shown)]) -> Json {
    Json::Arr(hs.iter().map(|(id, _)| Json::Str(id_text(id))).collect())
}
fn id_map(ts: &[(String, Vec<(Digest, Shown)>)]) -> Json {
    Json::Obj(ts.iter().map(|(t, hs)| (t.clone(), ids_json(hs))).collect())
}

/// Lean: `Loops.GroupValue.result`. One group's value, or a slot's without `by`.
fn group_result(v: &GroupValue) -> Json {
    match v {
        GroupValue::Nat(r) => obj(vec![
            ("unread", nat_json(&r.unread)),
            ("value", nat_json(&r.value)),
        ]),
        GroupValue::Int(r) => obj(vec![
            ("unread", nat_json(&r.unread)),
            ("value", int_json(&r.value)),
        ]),
        GroupValue::Opt(r) => obj(vec![
            ("unread", nat_json(&r.unread)),
            ("value", r.value.as_ref().map_or(Json::Null, int_json)),
        ]),
        GroupValue::Heads { heads, unread } => obj(vec![
            ("heads", ids_json(heads)),
            ("unread", nat_json(unread)),
        ]),
    }
}

/// Lean: `Loops.KeyedView.result`. A thing's heads, keys and whole-read emits.
fn keyed_result(k: &KeyedView) -> Json {
    obj(vec![
        ("heads", ids_json(&k.heads)),
        ("keys", id_map(&k.keys)),
        ("whole", ids_json(&k.whole)),
    ])
}

/// Lean: `Loops.SlotValue.result`. A slot's value in the canonical result.
pub fn slot_result(v: &SlotValue) -> Json {
    match v {
        SlotValue::Nat(r) => group_result(&GroupValue::Nat(r.clone())),
        SlotValue::Int(r) => group_result(&GroupValue::Int(r.clone())),
        SlotValue::Opt(r) => group_result(&GroupValue::Opt(r.clone())),
        SlotValue::Heads { heads, unread } => obj(vec![
            ("heads", ids_json(heads)),
            ("unread", nat_json(unread)),
        ]),
        SlotValue::Things { things, unread } => obj(vec![
            ("things", id_map(things)),
            ("unread", nat_json(unread)),
        ]),
        SlotValue::Keyed { things, unread } => obj(vec![
            (
                "things",
                Json::Obj(
                    things
                        .iter()
                        .map(|(t, k)| (t.clone(), keyed_result(k)))
                        .collect(),
                ),
            ),
            ("unread", nat_json(unread)),
        ]),
        SlotValue::Groups {
            groups,
            unread,
            ungrouped,
        } => obj(vec![
            (
                "groups",
                Json::Obj(
                    groups
                        .iter()
                        .map(|(g, v)| (g.clone(), group_result(v)))
                        .collect(),
                ),
            ),
            ("ungrouped", nat_json(ungrouped)),
            ("unread", nat_json(unread)),
        ]),
    }
}

/// Lean: `Loops.resultJson`. A vertex's whole result: every slot, by name.
pub fn result_json(r: &[(String, SlotValue)]) -> Json {
    Json::Obj(r.iter().map(|(n, v)| (n.clone(), slot_result(v))).collect())
}

/// Lean: `Loops.resultText`. JCS of the canonical result.
pub fn result_text(r: &[(String, SlotValue)]) -> String {
    result_json(r).canon()
}

/// Lean: `Loops.resultPreimage`. The label, then the canonical bytes.
pub fn result_preimage(r: &[(String, SlotValue)]) -> Vec<u8> {
    let mut bytes = RESULT_LABEL.as_bytes().to_vec();
    bytes.extend_from_slice(result_text(r).as_bytes());
    bytes
}

/// Lean: `Loops.resultHash`. `h:` and base32hex of `h` over the preimage.
/// The model does not require SHA-256 here; the vectors use it.
pub fn result_hash(h: &impl Fn(&[u8]) -> Vec<u8>, r: &[(String, SlotValue)]) -> String {
    format!("h:{}", base32::base32(&h(&result_preimage(r))))
}

#[cfg(test)]
mod tests {
    use super::{RESULT_LABEL, int_of, int_text};
    use crate::number::Int;

    #[test]
    fn integers_read_back_exactly_and_nothing_else() {
        for text in [
            "0",
            "7",
            "-7",
            "10",
            "9007199254740993",
            "-18014398509481984",
            "123456789012345678901234567890",
        ] {
            let i: Int = text.parse().expect("integer");
            assert_eq!(int_text(&i), text);
            assert_eq!(int_of(text), Some(i));
        }
        for text in [
            "", "-", "-0", "007", "00", "+1", "1:", "1e3", " 1", "1 ", "1.0", "--1", "0x10",
        ] {
            assert_eq!(int_of(text), None, "{text}");
        }
    }

    #[test]
    fn the_label_is_no_salt_length() {
        assert_eq!(RESULT_LABEL.len(), 21);
        assert_eq!(&RESULT_LABEL.as_bytes()[16..], b"h/v1:");
    }
}
