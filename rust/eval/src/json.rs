//! RFC 8259 ingestion, preserving object order and repeated names.
use crate::Error;
use loops::{json::Json, number::Number};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use std::fmt;

/// `OVERFLOW` selects what a lexeme past binary64's range becomes: an error,
/// or the stand-in [`out_of_range`] gives.
struct Raw<const OVERFLOW: bool>(Json);
impl<'de, const OVERFLOW: bool> Deserialize<'de> for Raw<OVERFLOW> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // RawValue avoids serde_json's reserved number-map collision and gives
        // each numeric lexeme directly to binary64 parsing, exactly once.
        let raw = Box::<serde_json::value::RawValue>::deserialize(d)?;
        let text = raw.get();
        if matches!(text.as_bytes().first(), Some(b'-' | b'0'..=b'9')) {
            let n: Number = match text.parse() {
                Ok(n) => n,
                Err(error) => match out_of_range(text) {
                    Some(n) if OVERFLOW => n,
                    _ => return Err(de::Error::custom(error)),
                },
            };
            return Ok(Self(n.whole().map_or(Json::Number(n), Json::Int)));
        }
        let mut parser = serde_json::Deserializer::from_str(text);
        parser
            .deserialize_any(RawVisitor::<OVERFLOW>)
            .map_err(de::Error::custom)
    }
}

/// The stand-in for a lexeme that overflows binary64 (D3).
///
/// The core's `Number` is finite, so it cannot hold what Lean's reader keeps
/// until the writer's number check. This adapter carries the overflow as the
/// largest finite magnitude of the same sign. It does so only for positions
/// whose every reader treats any magnitude above 2^53 − 1 alike: a payload,
/// which the writer refuses with `number-out-of-range` in its usual place,
/// and a spec's arguments, which the spec parser refuses. A refused draft or
/// spec is never canonicalized, so the stand-in never reaches bytes.
fn out_of_range(text: &str) -> Option<Number> {
    let value = text.parse::<f64>().ok()?;
    if !value.is_infinite() {
        return None;
    }
    Number::new(f64::MAX.copysign(value)).ok()
}

struct RawVisitor<const OVERFLOW: bool>;
impl<'de, const OVERFLOW: bool> Visitor<'de> for RawVisitor<OVERFLOW> {
    type Value = Raw<OVERFLOW>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RFC 8259 JSON with finite binary64 numbers")
    }
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Raw(Json::Null))
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Raw(Json::Bool(v)))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Raw(Json::Str(v.into())))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
        let mut xs = Vec::new();
        while let Some(Raw::<OVERFLOW>(v)) = a.next_element()? {
            xs.push(v);
        }
        Ok(Raw(Json::Arr(xs)))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
        let mut xs = Vec::new();
        while let Some((k, Raw::<OVERFLOW>(v))) = a.next_entry()? {
            xs.push((k, v));
        }
        Ok(Raw(Json::Obj(xs)))
    }
}

/// Parse RFC 8259 JSON, keeping duplicate members and their written order.
///
/// Numeric lexemes round once to finite binary64; safe-whole values become
/// `Json::Int`. Nonfinite results, malformed JSON, and lone surrogates fail.
/// This is an input decoder, not an exact-integer output decoder.
///
/// # Errors
/// Returns a location-bearing JSON error or a nonfinite-number refusal.
pub fn parse_json(text: &str) -> Result<Json, Error> {
    let mut d = serde_json::Deserializer::from_str(text);
    let raw = Raw::<false>::deserialize(&mut d)?;
    d.end()?;
    Ok(raw.0)
}

/// [`parse_json`], except that a lexeme past binary64's range is carried to
/// its reader as an out-of-range number instead of failing here. See
/// [`out_of_range`] for where that is sound.
pub(crate) fn parse_overflowing(text: &str) -> Result<Json, Error> {
    let mut d = serde_json::Deserializer::from_str(text);
    let raw = Raw::<true>::deserialize(&mut d)?;
    d.end()?;
    Ok(raw.0)
}

/// Decode a vector's `input` as its layer reads numbers (D3).
///
/// A record's and a batch's payloads reach the writer's number check, and a
/// spec reaches the spec parser, so overflow is carried there. A reader's
/// records refuse a nonfinite number as malformed input, so `spec/run`'s
/// `orders` and every other layer keep the strict decoder.
pub(crate) fn parse_input(layer: &str, text: &str) -> Result<Json, Error> {
    match layer.split('/').next() {
        Some("record" | "batch") => parse_overflowing(text),
        Some("spec") => {
            let Envelope(entries) = serde_json::from_str(text)?;
            let members = entries
                .into_iter()
                .map(|(name, raw)| {
                    let value = if name == "orders" {
                        parse_json(raw.get())
                    } else {
                        parse_overflowing(raw.get())
                    };
                    value.map(|json| (name, json))
                })
                .collect::<Result<_, _>>()?;
            Ok(Json::Obj(members))
        }
        _ => parse_json(text),
    }
}

/// The protocol's refusal of a JSON text, if it has one (D2): the text is
/// JSON by its syntax, and a string or member name in it holds a lone
/// surrogate. Text that is not JSON has no refusal, wherever a surrogate
/// stands in it. Lean: `Gen.Read.textRefusal`, model/Gen/Read.lean.
pub(crate) fn text_refusal(text: &str) -> Option<&'static str> {
    // IgnoredAny checks every token's syntax without decoding a string's
    // escapes into characters, so it accepts exactly the lone surrogate that
    // the decoding parse then refuses. Overflow is no fault to either.
    let syntax = serde_json::from_str::<de::IgnoredAny>(text).is_ok();
    (syntax && parse_overflowing(text).is_err()).then_some(loops::json::NOT_UNICODE)
}

// Walk raw metadata without converting numeric lexemes, while asking serde_json
// to decode every string and object key as Unicode scalar values.
struct UnicodeChecked;
impl<'de> Deserialize<'de> for UnicodeChecked {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = Box::<serde_json::value::RawValue>::deserialize(d)?;
        match raw.get().as_bytes().first() {
            Some(b'"') => {
                serde_json::from_str::<String>(raw.get()).map_err(de::Error::custom)?;
            }
            Some(b'[' | b'{') => {
                let mut parser = serde_json::Deserializer::from_str(raw.get());
                parser
                    .deserialize_any(UnicodeVisitor)
                    .map_err(de::Error::custom)?;
            }
            _ => {} // RawValue already checked syntax; numbers must remain untouched.
        }
        Ok(Self)
    }
}

struct UnicodeVisitor;
impl<'de> Visitor<'de> for UnicodeVisitor {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON array or object")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<(), A::Error> {
        while a.next_element::<UnicodeChecked>()?.is_some() {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<(), A::Error> {
        while a.next_entry::<String, UnicodeChecked>()?.is_some() {}
        Ok(())
    }
}

pub(crate) fn validate_metadata(
    raw: &serde_json::value::RawValue,
) -> Result<(), serde_json::Error> {
    serde_json::from_str::<UnicodeChecked>(raw.get()).map(|_| ())
}

/// An envelope holds raw members so metadata (especially `expect`) is never
/// used in the computation. The JSON syntax still must be valid.
pub(crate) struct Envelope(pub Vec<(String, Box<serde_json::value::RawValue>)>);
impl<'de> Deserialize<'de> for Envelope {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = Envelope;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a vector object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Envelope, A::Error> {
                let mut entries: Vec<(String, Box<serde_json::value::RawValue>)> = Vec::new();
                while let Some((k, v)) = a.next_entry()? {
                    if entries.iter().any(|(earlier, _)| earlier == &k) {
                        return Err(de::Error::custom(format!(
                            "the vector: duplicate member {k}"
                        )));
                    }
                    entries.push((k, v));
                }
                Ok(Envelope(entries))
            }
        }
        d.deserialize_map(Entries)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_input, parse_json, parse_overflowing, text_refusal};
    use loops::json::Json;

    #[test]
    fn lone_surrogates_are_refused_by_name_only_in_json_text() {
        for text in [
            r#""\ud800""#,
            r#""\udc00""#,
            r#""\udc00\ud800""#,
            r#""\ud83dx""#,
            r#""\ud83d\u0041""#,
            r#"{"\udfff":0}"#,
            r#"[["\ud83d\ud83d\ude00"]]"#,
            r#"{"a":1,"a":2,"b":"\uD800"}"#,
            r#"["\ud800\n", "\udc00"]"#,
            r#"{"n":1e999,"b":"\ud800"}"#,
        ] {
            assert_eq!(text_refusal(text), Some("string-not-unicode"), "{text}");
            assert!(parse_json(text).is_err(), "{text}");
        }
        for text in [
            r#""\ud83d\ude00""#,
            r#""\uD83D\uDE00""#,
            r#"{"a" "\ud800"}"#,
            "[1,]",
            r#""\ud83d\uZZZZ""#,
            r#"{"a":"\ud800""#,
            r#""\ud800"#,
            r#"["\ud800",]"#,
            r#""\ud800" 1"#,
            r#"["\udc00", 01]"#,
            "1e999",
            r#"{"a":1,"a":2}"#,
        ] {
            assert_eq!(text_refusal(text), None, "{text}");
        }
        assert_eq!(
            loops::json::of_utf16(&[0xD83D, 0xDE00]).as_deref(),
            Some("😀")
        );
        for units in [
            &[0xD800][..],
            &[0xDC00],
            &[0xDC00, 0xD800],
            &[0xD83D, 0x41],
            &[0x41, 0xD83D],
            &[0xD83D, 0xD83D, 0xDE00],
            &[0xD83D, 0xDE00, 0xDE00],
        ] {
            assert_eq!(loops::json::of_utf16(units), None, "{units:x?}");
        }
        for s in [
            "",
            "plain",
            "雪😀",
            "\u{10ffff}",
            "\u{d7ff}\u{e000}\u{ffff}",
        ] {
            let units: Vec<u16> = s.encode_utf16().collect();
            assert_eq!(loops::json::of_utf16(&units).as_deref(), Some(s));
        }
    }

    #[test]
    fn overflow_is_carried_only_where_its_reader_refuses_it() {
        for (text, sign) in [("1e999", 1.0), ("-1e999", -1.0), ("1.5E+400", 1.0)] {
            assert!(parse_json(text).is_err(), "{text}");
            let Json::Number(n) = parse_overflowing(text).expect("carried") else {
                panic!("a number")
            };
            assert!(!n.in_range());
            assert_eq!(f64::from_bits(n.bits()), f64::MAX * sign);
        }
        // Only overflow is carried: grammar and nonfinite words still fail.
        for bad in [
            "NaN",
            "Infinity",
            "-Infinity",
            "01",
            "1.",
            "1e",
            "+1",
            "[1e999,]",
        ] {
            assert!(parse_overflowing(bad).is_err(), "{bad}");
        }
        assert_eq!(
            parse_overflowing("[1,1.5,1e2]").expect("finite"),
            parse_json("[1,1.5,1e2]").expect("finite")
        );
        let record = r#"{"envs":[{"kind":1e999}],"payloads":[1e999],"salt":1e999}"#;
        assert!(parse_input("record", record).is_ok());
        assert!(parse_input("batch", r#"{"held":[],"drafts":[{"payload":-1e999}]}"#).is_ok());
        assert!(parse_input("spec/parse", r#"{"spec":{"fold":1e999}}"#).is_ok());
        assert!(parse_input("spec/run", r#"{"spec":1e999,"orders":[]}"#).is_ok());
        assert!(parse_input("spec/run", r#"{"spec":{},"orders":[[{"n":1e999}]]}"#).is_err());
        assert!(parse_input("spec/run", r#"{"spec":{},"spec":{},"orders":[]}"#).is_err());
        assert!(parse_input("spec/parse", "[]").is_err());
        for layer in [
            "fold/sum",
            "realm",
            "scenario",
            "lens/section",
            "byte-text",
            "other",
        ] {
            assert!(parse_input(layer, r#"{"x":[1e999]}"#).is_err(), "{layer}");
            assert!(parse_input(layer, r#"{"x":[1]}"#).is_ok(), "{layer}");
        }
    }

    #[test]
    fn raw_members_surrogates_and_number_grammar() {
        let j = parse_json(r#"{"z":0,"a":1,"z":2,"$serde_json::private::Number":"12"}"#)
            .expect("raw members");
        let Json::Obj(kvs) = j else { panic!("object") };
        assert_eq!(
            kvs.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            ["z", "a", "z", "$serde_json::private::Number"]
        );
        for bad in [
            "NaN",
            "Infinity",
            "-Infinity",
            "1e999",
            "-1e999",
            "01",
            "1.",
            "1e",
            "+1",
            "[1,]",
            r#""\ud800""#,
            r#""\udc00""#,
            "0 1",
        ] {
            assert!(parse_json(bad).is_err(), "{bad}");
        }
        assert_eq!(
            parse_json(r#""\ud834\udd1e""#).expect("pair"),
            Json::Str("𝄞".into())
        );
        for raw in ["-0", "-0.0", "-0e20", "0.0", "1e-999"] {
            assert_eq!(parse_json(raw).expect("zero").canon(), "0");
        }
        for (raw, expected) in [
            ("1.0", "1"),
            ("1e2", "100"),
            ("9007199254740993", "9007199254740992"),
            ("0.10000000000000001", "0.1"),
        ] {
            assert_eq!(parse_json(raw).expect("number").canon(), expected);
        }
    }
}
