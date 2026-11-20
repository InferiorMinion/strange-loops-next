//! Gen.ResultVectors, model/Gen/Result.lean: a `spec/run` input's whole
//! result as canonical text, and its hash with SHA-256.
use super::spec::runs;
use loops::{
    json::Json,
    result::{result_hash, result_text},
};
use sha2::{Digest, Sha256};

/// Gen.ResultVectors.resultExpect: `{"canonical", "hash"}` for each order.
pub(crate) fn answers(layer: &str, input: &Json) -> Result<Vec<Json>, String> {
    if layer != "result" {
        return Err(format!("unknown result layer {layer}"));
    }
    let hash = |b: &[u8]| Sha256::digest(b).to_vec();
    Ok(runs(input)?
        .iter()
        .map(|r| {
            Json::Obj(vec![
                ("canonical".into(), Json::Str(result_text(r))),
                ("hash".into(), Json::Str(result_hash(&hash, r))),
            ])
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::answers;
    use crate::parse;

    #[test]
    fn closed_dispatch_and_strict_input() {
        for (layer, raw) in [
            (
                "result/future",
                r#"{"spec":{"vertex":"x","from":[{"store":"."}]},"orders":[[]]}"#,
            ),
            (
                "result",
                r#"{"spec":{"vertex":"x","from":[{"store":"."}]},"orders":[]}"#,
            ),
            (
                "result",
                r#"{"spec":{"vertex":"x","from":[{"store":"."}]},"orders":[[{}]]}"#,
            ),
            (
                "result",
                r#"{"spec":{"vertex":"x","from":[{"store":"."}]}}"#,
            ),
        ] {
            assert!(answers(layer, &parse(raw)).is_err(), "{layer} {raw}");
        }
    }
}
