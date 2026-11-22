//! Input-only fixture evaluator for the `Gen.Run` families, `result` among them.
//!
//! The pure `loops` crate owns the model. This boundary owns RFC 8259
//! decoding, fixture shape checks, SHA-256 and Ed25519. It does not read a
//! fixture's `expect`, route sources, or perform storage/network operations.
//! Arrival-order answers remain separate until agreement is checked.
mod batch;
mod byte_text;
mod fold;
mod json;
mod lens;
mod payload;
pub use payload::fixture_fields;
mod reader;
mod realm;
mod record;
mod result;
mod scenario;
mod spec;
mod tick;

pub use json::parse_json;
use loops::json::Json;
use std::fmt;
pub use tick::supports_tick_op;

/// A JSON syntax, fixture decoding, or arrival-order disagreement error.
/// Writer refusals are successful answers containing `refused`, not errors.
/// So are a text payload's lone surrogate and a lens base that repeats an id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<String> for Error {
    fn from(message: String) -> Self {
        Self(message)
    }
}
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self(error.to_string())
    }
}

/// Renders a spec's slot values as the `spec/run` vectors write them.
pub fn slots_json(values: Vec<(String, loops::spec::SlotValue)>) -> Json {
    Json::Obj(
        values
            .into_iter()
            .map(|(name, value)| (name, spec::slot(value)))
            .collect(),
    )
}

/// Compute every arrival-order (or realm rotation) answer.
///
/// Accepts `record`, `fold/*`, `spec/{parse,run}`, `realm`, `scenario`,
/// `lens/section`, `batch`, `byte-text`, `payload`, `result` and `tick`. No expected answer
/// is supplied. Keeping all answers makes disagreement observable to callers.
/// A record input gives one answer: its first envelope and first payload
/// decide, and a later variant that disagrees once they are written is an error.
///
/// # Errors
/// Returns malformed input/unknown-layer errors. It does not silently select
/// one answer when orders differ; use [`evaluate`] for checked agreement.
pub fn answers(layer: &str, input: &Json) -> Result<Vec<Json>, Error> {
    let run = match layer.split('/').next() {
        Some("payload") => payload::answers,
        Some("record") => record::answers,
        Some("fold") => fold::answers,
        Some("spec") => spec::answers,
        Some("realm") => realm::answers,
        Some("scenario") => scenario::answers,
        Some("lens") => lens::answers,
        Some("batch") => batch::answers,
        Some("byte-text") => byte_text::answers,
        Some("result") => result::answers,
        Some("tick") => tick::answers,
        _ => return Err(Error(format!("layer: unknown layer {layer}"))),
    };
    run(layer, input).map_err(Error::from)
}

/// Evaluate a vector's `layer` and `input` only, checking all answers agree.
///
/// A number lexeme past binary64's range is malformed input to a reader's
/// records. In a record's or a batch's payload it reaches the writer, which
/// refuses it with `number-out-of-range`; in a spec it reaches the spec
/// parser, which refuses it as it does any number out of range.
///
/// Metadata, including `expect`, is not used as an answer. Its strings are
/// validated as Unicode scalars without converting numbers. The envelope must be a
/// syntactically valid JSON object with distinct member names. Unknown
/// metadata is allowed, matching `Gen.Run.readVector`.
///
/// # Errors
/// Returns JSON/shape errors, empty answer lists, or differing order answers.
pub fn evaluate(text: &str) -> Result<Json, Error> {
    let envelope: json::Envelope = serde_json::from_str(text)?;
    for (name, value) in &envelope.0 {
        if name != "layer" && name != "input" {
            json::validate_metadata(value)?;
        }
    }
    let required = |name: &str| {
        envelope
            .0
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.get())
            .ok_or_else(|| Error(format!("the vector: missing {name}")))
    };
    let layer: String = serde_json::from_str(required("layer")?)?;
    let input = json::parse_input(&layer, required("input")?)?;
    let mut values = answers(&layer, &input)?.into_iter();
    let first = values
        .next()
        .ok_or_else(|| Error("input: no arrival order given".into()))?;
    let canonical = first.canon();
    for (i, value) in values.enumerate() {
        if value.canon() != canonical {
            return Err(Error(format!(
                "input.orders[{}]: gives a different answer than input.orders[0]",
                i + 1
            )));
        }
    }
    Ok(first)
}

#[cfg(test)]
fn parse(text: &str) -> Json {
    parse_json(text).expect("test input JSON")
}

#[cfg(test)]
mod tests {
    use super::{answers, evaluate, parse_json};

    #[test]
    fn expect_is_never_decoded_or_used() {
        for metadata in [
            "",
            r#", "expect":1e999"#,
            r#", "expect":{"arbitrary":[true,null]}"#,
        ] {
            let raw = format!(r#"{{"layer":"fold/count","input":{{"orders":[[]]}}{metadata}}}"#);
            assert_eq!(evaluate(&raw).expect("input-only").canon(), "0");
        }
        assert!(
            evaluate(r#"{"layer":"fold/count","layer":"fold/sum","input":{"orders":[[]]}}"#)
                .is_err()
        );
        assert!(evaluate(r#"{"layer":"future","input":{},"expect":0}"#).is_err());
    }

    #[test]
    fn ignored_metadata_requires_unicode_scalars() {
        for metadata in [
            r#""expect":"\ud800""#,
            r#""expect":"\udc00""#,
            r#""other":[{"nested":["\ud800"]}]"#,
            r#""expect":{"nested":{"\udc00":0}}"#,
        ] {
            let raw = format!(r#"{{"layer":"fold/count","input":{{"orders":[[]]}},{metadata}}}"#);
            assert!(evaluate(&raw).is_err(), "{metadata}");
        }
        for metadata in [
            r#""expect":"\ud834\udd1e""#,
            r#""expect":{"\ud834\udd1e":[1e999,{"x":1,"x":2}]}"#,
            r#""expect":1e999"#,
        ] {
            let raw = format!(r#"{{"layer":"fold/count","input":{{"orders":[[]]}},{metadata}}}"#);
            assert_eq!(
                evaluate(&raw).expect("valid metadata").canon(),
                "0",
                "{metadata}"
            );
        }
    }

    #[test]
    fn differing_orders_are_not_silently_selected() {
        let form = r#"{"id":"r:0000000000000000000000000000000000000000000000000000","kind":"","observer":"","about":[],"prev":[],"emitted":"2026-01-01T00:00:00Z"}"#;
        let input = format!(r#"{{"orders":[[],[{form}]]}}"#);
        assert_eq!(
            answers("fold/count", &parse_json(&input).expect("JSON"))
                .expect("answers")
                .len(),
            2
        );
        let raw = format!(r#"{{"layer":"fold/count","input":{input}}}"#);
        assert!(
            evaluate(&raw)
                .expect_err("different answers")
                .to_string()
                .contains("different answer")
        );
        assert!(evaluate(r#"{"layer":"fold/count","input":{"orders":[]}}"#).is_err());
    }
}
