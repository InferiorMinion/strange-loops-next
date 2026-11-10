//! Gen.Records.recordAnswer: the first envelope and payload decide, later
//! variants must agree once those are written, and the seed is read last.
use super::{
    json::{parse_overflowing, text_refusal},
    reader::{bytes, list, members, optional, required, text},
};
use ed25519_dalek::{Signer, SigningKey};
use loops::{base32, envelope::Env, json::Json, stored};
use sha2::{Digest, Sha256};

fn hash(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}
fn object(kvs: Vec<(&str, Json)>) -> Json {
    Json::Obj(kvs.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn string(s: impl Into<String>) -> Json {
    Json::Str(s.into())
}

fn refused(code: &str) -> Json {
    object(vec![("refused", string(code))])
}

/// The written record's answer. `commit` and `id` are the checked head's.
fn written(e: &Env, salt: &[u8], payload: &Json, commit: &str, id: &str, seed: &[u8; 32]) -> Json {
    let key = SigningKey::from_bytes(seed);
    let key_text = format!("k:{}", base32::base32(&key.verifying_key().to_bytes()));
    let sig = base32::base32(&key.sign(&stored::signing_input(id)).to_bytes());
    let signature = object(vec![("key", string(&key_text)), ("ed25519", string(&sig))]);
    let line = object(vec![
        ("id", string(id)),
        ("env", e.to_json()),
        ("salt", string(base32::base32(salt))),
        ("payload", payload.clone()),
        ("sig", signature.clone()),
    ]);
    let erased_line = object(vec![
        ("id", string(id)),
        ("env", e.to_json()),
        ("commit", string(commit)),
        ("sig", signature),
    ]);
    object(vec![
        ("env_canonical", string(e.to_json().canon())),
        ("payload_canonical", string(payload.canon())),
        ("commit", string(commit)),
        ("id_preimage", string(stored::id_preimage(e, commit))),
        ("id", string(id)),
        ("signing_input", string(format!("loops/record/v1:{id}"))),
        ("key", string(key_text)),
        ("ed25519", string(sig)),
        ("line", string(line.canon())),
        ("erased_line", string(erased_line.canon())),
    ])
}

pub(crate) fn answers(layer: &str, input: &Json) -> Result<Vec<Json>, String> {
    match answers_decoded(layer, input) {
        Err(e) if e == "envelope-field-unknown" => {
            Ok(vec![Json::Obj(vec![("refused".into(), Json::Str(e))])])
        }
        other => other,
    }
}

fn answers_decoded(layer: &str, input: &Json) -> Result<Vec<Json>, String> {
    if layer != "record" {
        return Err(format!("unknown record layer {layer}"));
    }
    let kvs = members(
        "input",
        &["envs", "payloads", "payload_text", "salt", "seed"],
        input,
    )?;
    // Raw shape comes first, across every envelope.
    let shaped = loops::envelope::read_all("input.envs", required("input", kvs, "envs")?)
        .map_err(|e| e.to_string());
    if let Err(e) = &shaped
        && e == "envelope-field-unknown"
    {
        return Err(e.clone());
    }
    // D2: then a text payload's lone surrogate, before any envelope value,
    // the salt or the payload's members are looked at.
    if let Some(Json::Str(raw)) = optional(kvs, "payload_text")
        && let Some(code) = text_refusal(raw)
    {
        return Ok(vec![refused(code)]);
    }
    let envs = shaped?;
    // D3: a payload number past binary64's range reaches the writer's number check.
    let payloads = match (optional(kvs, "payloads"), optional(kvs, "payload_text")) {
        (Some(ps), None) => list("input.payloads", ps)?.to_vec(),
        (None, Some(raw)) => vec![
            parse_overflowing(text("input.payload_text", raw)?)
                .map_err(|e| format!("input.payload_text: {e}"))?,
        ],
        _ => return Err("input: takes one of payloads and payload_text".into()),
    };
    let salt = bytes("input.salt", required("input", kvs, "salt")?)?;
    let Some(env) = envs.first() else {
        return Err("input.envs: no envelope given".into());
    };
    let Some(payload) = payloads.first() else {
        return Err("input.payloads: no payload given".into());
    };
    // D5: the first envelope and first payload are written; their refusal is
    // the answer, whatever the other variants are.
    let w = match stored::write(env.clone(), salt.clone(), payload.clone()) {
        Ok(w) => w,
        Err(e) => return Ok(vec![refused(e.code())]),
    };
    let r = w.stored();
    let commit = r.commit(&hash);
    let id = r.id(&hash);
    #[cfg(test)]
    tests::assert_opening_laws(&r, &commit, &salt, payload, &id);
    // Once they are written, a variant that names another record, or a payload
    // that is refused, is a fault in the input and never an answer.
    for (i, e) in envs.iter().enumerate() {
        if stored::id_of(&hash, e, &commit) != id {
            return Err(format!(
                "input.envs[{i}]: names another record than input.envs[0]"
            ));
        }
    }
    for (i, p) in payloads.iter().enumerate() {
        if let Some(why) = stored::write_problem(env, &salt, p) {
            return Err(format!(
                "input.payloads[{i}]: is refused ({why}) where input.payloads[0] is written"
            ));
        }
        if stored::commit_of(&hash, &salt, p) != commit {
            return Err(format!(
                "input.payloads[{i}]: commits to another payload than input.payloads[0]"
            ));
        }
    }
    // D4: only now is the seed read. It is exactly 32 bytes; none is padded or cut.
    let seed: [u8; 32] = bytes("input.seed", required("input", kvs, "seed")?)?
        .try_into()
        .map_err(|seed: Vec<u8>| {
            format!(
                "input.seed: holds {} bytes; a signing seed is exactly 32",
                seed.len()
            )
        })?;
    Ok(vec![written(env, &salt, payload, &commit, &id, &seed)])
}

#[cfg(test)]
mod tests {
    use super::{answers, hash, refused};
    use crate::{evaluate, parse};
    use loops::{json::Json, stored};

    // Preserve the original vector runner's independent stored-record laws,
    // keeping assertions outside the fallible production boundary.
    pub(super) fn assert_opening_laws(
        r: &stored::Stored,
        commit: &str,
        salt: &[u8],
        payload: &Json,
        id: &str,
    ) {
        assert!(stored::opens(&hash, commit, salt, payload));
        let erased = r.erase(&hash);
        assert_eq!(erased.id(&hash), id);
        assert_eq!(erased.erase(&hash), erased);
    }

    #[test]
    fn every_record_variant_keeps_openings_and_erasure_laws() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vectors/record");
        let mut files = std::fs::read_dir(root)
            .expect("record fixtures")
            .map(|entry| entry.expect("record entry").path())
            .filter(|path| path.extension().is_some_and(|e| e == "json"))
            .collect::<Vec<_>>();
        files.sort();
        let mut failures = Vec::new();
        for path in files {
            let raw = std::fs::read_to_string(&path).expect("record fixture");
            if let Err(error) = evaluate(&raw) {
                failures.push(format!("{path:?}: {error}"));
            }
        }
        assert_eq!(
            failures,
            Vec::<String>::new(),
            "all record variants executed"
        );
    }

    #[test]
    fn all_shapes_precede_envelope_values_and_salt() {
        for envs in [
            r#"[{"kind":7},{"extra":1}]"#,
            r#"[{"extra":1},{"kind":7}]"#,
            r#"[7,{"extra":1}]"#,
            r#"[{"extra":1},7]"#,
            r#"[{"extra":1}]"#,
            r#"[{"kind":7,"extra":1}]"#,
        ] {
            let input = parse(&format!(
                r#"{{"envs":{envs},"salt":"!","payloads":[null]}}"#
            ));
            assert_eq!(
                answers("record", &input).expect("shape refusal"),
                vec![parse(r#"{"refused":"envelope-field-unknown"}"#)]
            );
        }
    }

    const ENV: &str = r#"{"kind":"note","observer":"o:test","emitted":"2026-10-21T00:00:00Z"}"#;
    const NO_KIND: &str = r#"{"kind":"","observer":"o:test","emitted":"2026-10-21T00:00:00Z"}"#;
    const UNTYPED: &str = r#"{"kind":"note","observer":"test","emitted":"2026-10-21T00:00:00Z"}"#;
    const EXTRA: &str =
        r#"{"kind":"note","observer":"o:test","emitted":"2026-10-21T00:00:00Z","x":0}"#;
    const SALT: &str = "00000000000000000000000000";
    const SHORT_SALT: &str = "000000000000000000000000";

    /// One record input as the evaluator decodes it: its answer, or the fault.
    fn answer(envs: &str, payloads: &str, salt: &str, seed: Option<&str>) -> Result<Json, String> {
        let seed = seed.map_or_else(String::new, |s| format!(r#","seed":"{s}""#));
        let raw = format!(r#"{{"envs":[{envs}],{payloads},"salt":"{salt}"{seed}}}"#);
        let input = crate::json::parse_input("record", &raw).map_err(|e| e.to_string())?;
        match answers("record", &input)?.as_slice() {
            [one] => Ok(one.clone()),
            many => Err(format!("a record input has one answer, not {}", many.len())),
        }
    }
    fn zeros(n: usize) -> String {
        "0".repeat(n)
    }

    #[test]
    fn a_lone_surrogate_in_text_follows_shape_and_precedes_values() {
        let lone = r#""payload_text":"\"\\ud800\"""#;
        for (envs, salt) in [
            (ENV.to_owned(), SALT),
            (NO_KIND.to_owned(), SALT),
            (format!("{UNTYPED},{NO_KIND}"), SALT),
            (ENV.to_owned(), SHORT_SALT),
            (ENV.to_owned(), "!"),
        ] {
            assert_eq!(
                answer(&envs, lone, salt, None),
                Ok(refused("string-not-unicode")),
                "{envs}"
            );
        }
        for envs in [
            EXTRA.to_owned(),
            format!("{ENV},{EXTRA}"),
            format!("{EXTRA},{NO_KIND}"),
        ] {
            assert_eq!(
                answer(&envs, lone, SALT, None),
                Ok(refused("envelope-field-unknown")),
                "{envs}"
            );
        }
        for text in [
            r#"{\"a\":1,\"a\":2,\"b\":\"\\udc00\"}"#,
            r#"{\"n\":1e999,\"\\ud800\":0}"#,
        ] {
            let payloads = format!(r#""payload_text":"{text}""#);
            assert_eq!(
                answer(ENV, &payloads, SALT, None),
                Ok(refused("string-not-unicode")),
                "{text}"
            );
        }
        // Text that is not JSON has no refusal, wherever the surrogate stands.
        for text in [r#"{\"a\" \"\\ud800\"}"#, r#"[\"\\ud800\""#, r#"\"\\ud800"#] {
            let payloads = format!(r#""payload_text":"{text}""#);
            assert!(answer(ENV, &payloads, SALT, None).is_err(), "{text}");
        }
        // A lone surrogate outside a text payload is no vector at all.
        let raw = format!(
            r#"{{"layer":"record","input":{{"envs":[{ENV}],"payloads":["\ud800"],"salt":"{SALT}"}}}}"#
        );
        assert!(evaluate(&raw).is_err());
    }

    #[test]
    fn overflow_is_the_writers_number_refusal_in_its_usual_place() {
        for payloads in [
            r#""payloads":[1e999]"#,
            r#""payloads":[{"n":[-1e999]}]"#,
            r#""payload_text":"1e999""#,
            r#""payload_text":"{\"n\":-1E400}""#,
        ] {
            assert_eq!(
                answer(ENV, payloads, SALT, None),
                Ok(refused("number-out-of-range")),
                "{payloads}"
            );
            assert_eq!(
                answer(NO_KIND, payloads, SALT, None),
                Ok(refused("kind-empty"))
            );
            assert_eq!(
                answer(ENV, payloads, SHORT_SALT, None),
                Ok(refused("salt-not-16-bytes"))
            );
            assert_eq!(
                answer(EXTRA, payloads, SALT, None),
                Ok(refused("envelope-field-unknown"))
            );
        }
        assert_eq!(
            answer(ENV, r#""payload_text":"{\"a\":1e999,\"a\":0}""#, SALT, None),
            Ok(refused("member-duplicate"))
        );
        // An overflowing number where text is wanted is malformed, after shape.
        let kind = r#"{"kind":1e999,"observer":"o:test","emitted":"2026-10-21T00:00:00Z"}"#;
        assert!(answer(kind, r#""payloads":[{}]"#, SALT, None).is_err());
        assert_eq!(
            answer(&format!("{kind},{EXTRA}"), r#""payloads":[{}]"#, SALT, None),
            Ok(refused("envelope-field-unknown"))
        );
    }

    #[test]
    fn a_seed_is_exactly_32_bytes_and_read_last() {
        let written = r#""payloads":[{}]"#;
        let good = answer(ENV, written, SALT, Some(&zeros(52))).expect("32-byte seed");
        assert!(matches!(&good, Json::Obj(kvs) if kvs.iter().any(|(k, _)| k == "ed25519")));
        let short = format!("{}1", zeros(51));
        for seed in [
            Some(zeros(50)),
            Some(zeros(53)),
            Some(String::new()),
            Some(short),
            Some("!".to_owned()),
            None,
        ] {
            let seed = seed.as_deref();
            assert!(answer(ENV, written, SALT, seed).is_err(), "{seed:?}");
            assert_eq!(
                answer(NO_KIND, written, SALT, seed),
                Ok(refused("kind-empty")),
                "a refused record does not wait on its seed: {seed:?}"
            );
        }
    }

    #[test]
    fn the_first_variant_decides_and_a_later_one_never_answers() {
        let (written, seed) = (r#""payloads":[{}]"#, zeros(52));
        let seed = Some(seed.as_str());
        let envs = |a: &str, b: &str| format!("{a},{b}");
        assert_eq!(
            answer(&envs(NO_KIND, ENV), written, SALT, seed),
            Ok(refused("kind-empty"))
        );
        assert_eq!(
            answer(&envs(NO_KIND, UNTYPED), written, SALT, seed),
            Ok(refused("kind-empty"))
        );
        // The first variant's refusal, not the earliest-ranked among variants.
        assert_eq!(
            answer(&envs(UNTYPED, NO_KIND), written, SALT, seed),
            Ok(refused("observer-not-typed"))
        );
        assert_eq!(
            answer(&envs(ENV, NO_KIND), r#""payloads":[1e999,{}]"#, SALT, seed),
            Ok(refused("number-out-of-range"))
        );
        assert_eq!(
            answer(
                &envs(NO_KIND, ENV),
                r#""payloads":[9007199254740992,{}]"#,
                SALT,
                seed
            ),
            Ok(refused("kind-empty"))
        );
        for (envs, payloads) in [
            (envs(ENV, NO_KIND), written),
            (envs(ENV, UNTYPED), written),
            (ENV.to_owned(), r#""payloads":[{},9007199254740992]"#),
            (ENV.to_owned(), r#""payloads":[null,1e999]"#),
            (ENV.to_owned(), r#""payloads":[{"a":1},{"a":1,"a":2}]"#),
            (ENV.to_owned(), r#""payloads":[{"a":1},{"a":2}]"#),
        ] {
            assert!(
                answer(&envs, payloads, SALT, seed).is_err(),
                "a later variant answers: {envs} {payloads}"
            );
        }
        let agreed = r#""payloads":[{"a":1,"b":2},{"b":2,"a":1}]"#;
        assert_eq!(
            answer(&envs(ENV, ENV), agreed, SALT, seed),
            answer(ENV, r#""payloads":[{"b":2,"a":1}]"#, SALT, seed)
        );
    }

    #[test]
    fn seed_and_variant_shapes_fail_without_panics() {
        let env = r#"{"kind":"note","observer":"sl:a","emitted":"2026-01-01T00:00:00Z","about":["thing:a"]}"#;
        for seed in [r#""""#, r#""a""#, r#""aa""#, r#""A""#, "null"] {
            let input = parse(&format!(
                r#"{{"envs":[{env}],"payloads":[null],"salt":"00000000000000000000000000","seed":{seed}}}"#
            ));
            assert!(answers("record", &input).is_err(), "{seed}");
        }
        for input in [
            r#"{"envs":[],"payloads":[null],"salt":""}"#,
            r#"{"envs":[],"payloads":[],"payload_text":"null","salt":""}"#,
            r#"{"envs":[],"payload_text":"[","salt":""}"#,
        ] {
            assert!(answers("record", &parse(input)).is_err());
        }
    }
}
