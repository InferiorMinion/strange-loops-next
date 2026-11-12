//! Stored lines against the model's record vectors, and what a reader refuses.
use loops::{envelope, json::Json, record::Shown, stored};
use loops_eval::parse_json;
use loops_place::{
    key::Key,
    line::{Line, Refusal},
};
use std::{fs, path::Path};

fn member<'a>(j: &'a Json, name: &str) -> Option<&'a Json> {
    match j {
        Json::Obj(members) => members.iter().find(|(k, _)| k == name).map(|(_, v)| v),
        _ => None,
    }
}
fn text<'a>(j: &'a Json, name: &str) -> &'a str {
    match member(j, name) {
        Some(Json::Str(s)) => s,
        _ => panic!("{name}: expected text"),
    }
}
/// Every accepted record vector, as (name, signed line, expect).
fn accepted() -> Vec<(String, Line, Json)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vectors/record");
    let mut paths: Vec<_> = fs::read_dir(dir)
        .expect("record vectors")
        .map(|e| e.expect("entry").path())
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for path in paths {
        let raw = fs::read_to_string(&path).expect("vector");
        // A number past binary64's range is not read here; such a vector is a refusal.
        let Ok(vector) = parse_json(&raw) else {
            assert!(raw.contains(r#""refused""#), "{}", path.display());
            continue;
        };
        let expect = member(&vector, "expect").expect("expect").clone();
        if member(&expect, "refused").is_some() {
            continue;
        }
        let input = member(&vector, "input").expect("input");
        let Some(Json::Arr(envs)) = member(input, "envs") else {
            panic!("envs")
        };
        // A vector gives its payload as values, or as one JSON text.
        let payloads = match (member(input, "payloads"), member(input, "payload_text")) {
            (Some(Json::Arr(payloads)), None) => payloads.clone(),
            (None, Some(Json::Str(raw))) => vec![parse_json(raw).expect("payload text")],
            _ => panic!("payloads"),
        };
        let salt = loops::base32::decode(text(input, "salt")).expect("salt");
        let key = Key::from_text(text(input, "seed")).expect("seed");
        // Every variant of a vector writes the same line.
        for env in envs {
            for payload in &payloads {
                let env = envelope::read("env", env).expect("envelope");
                let written = stored::write(env, salt.clone(), payload.clone()).expect("written");
                out.push((
                    path.display().to_string(),
                    Line::sign(&written, &key),
                    expect.clone(),
                ));
            }
        }
    }
    assert!(out.len() >= 14, "the accepted record vectors were found");
    out
}

#[test]
fn signed_lines_are_the_vectors_lines_and_read_back() {
    for (name, line, expect) in accepted() {
        assert_eq!(line.render(), text(&expect, "line"), "{name}");
        assert_eq!(line.id(), text(&expect, "id"), "{name}");
        assert_eq!(line.key(), text(&expect, "key"), "{name}");
        assert_eq!(
            Line::parse(&line.render()).map(|l| l.render()),
            Ok(line.render()),
            "{name}"
        );
        let erased = line.erased();
        assert_eq!(erased.render(), text(&expect, "erased_line"), "{name}");
        assert_eq!(
            Line::parse(&erased.render()).map(|l| l.render()),
            Ok(erased.render()),
            "{name}"
        );
        assert_eq!(erased.id(), line.id(), "{name}");
        assert_eq!(erased.erased(), erased, "{name}");
        assert_eq!(line.record().shown, Shown::Here, "{name}");
        assert_eq!(erased.record().shown, Shown::Erased, "{name}");
        assert_eq!(line.record().id, erased.record().id, "{name}");
        // A reader's record is of what the envelope's `of` names, as written.
        assert_eq!(line.record().of, line.env().of, "{name}");
        assert_eq!(erased.record().of, line.env().of, "{name}");
    }
}

#[test]
fn a_changed_line_is_refused_by_the_check_that_covers_the_change() {
    let (_, line, _) = accepted().swap_remove(1);
    let good = line.render();
    let other_key = Key::from_text(&"0".repeat(52)).expect("zero seed");
    let cases = [
        (
            "not json",
            good.replace("{\"env\"", "{env"),
            Refusal::NotJson,
        ),
        ("an array", "[]".to_owned(), Refusal::NotJson),
        (
            "an extra member",
            good.replacen('{', "{\"also\":1,", 1),
            Refusal::Shape,
        ),
        (
            "an unknown envelope field",
            good.replace("\"env\":{", "\"env\":{\"extra\":1,"),
            Refusal::Shape,
        ),
        (
            "an edited envelope",
            good.replace("\"kind\":\"", "\"kind\":\"x"),
            Refusal::IdMismatch,
        ),
        (
            "another key",
            good.replace(line.key(), &other_key.public()),
            Refusal::SignatureInvalid,
        ),
        (
            "a key that is not 32 bytes",
            good.replace(line.key(), "k:00"),
            Refusal::KeyInvalid,
        ),
        (
            "a second spelling",
            format!("{good} "),
            Refusal::NotCanonical,
        ),
    ];
    for (what, changed, refusal) in cases {
        assert_eq!(Line::parse(&changed), Err(refusal), "{what}");
    }
    // The id covers the payload through the commitment, and the commitment through the salt.
    let Json::Obj(mut members) = line.json() else {
        panic!("a line is an object")
    };
    for (name, value) in &mut members {
        if name == "payload" {
            *value = Json::Arr(vec![value.clone()]);
        }
    }
    assert_eq!(
        Line::parse(&Json::Obj(members).canon()),
        Err(Refusal::IdMismatch)
    );
    let erased = line.erased().render();
    assert_eq!(
        Line::parse(&erased.replace("\"commit\":\"h:", "\"commit\":\"x:")),
        Err(Refusal::CommitInvalid)
    );
    // Another commitment of the right shape is not this record's.
    let at = erased.find("\"commit\":\"h:").expect("commit") + 12;
    let flipped = if &erased[at..=at] == "0" { "1" } else { "0" };
    let other = format!("{}{flipped}{}", &erased[..at], &erased[at + 1..]);
    assert_eq!(Line::parse(&other), Err(Refusal::IdMismatch));
}

#[test]
fn a_mistyped_envelope_field_in_a_line_gets_the_writers_code() {
    let (_, line, _) = accepted().swap_remove(1);
    // The core's envelope reader reads a mistyped or missing field as unreadable
    // text, so the writer's check refuses it, before the id is compared; an
    // envelope that is not an object stays a line shape refusal.
    for good in [line.render(), line.erased().render()] {
        let kind = good.find("\"kind\":\"").expect("kind");
        let end = kind + 8 + good[kind + 8..].find('"').expect("kind end");
        let mistyped = format!("{}\"kind\":7{}", &good[..kind], &good[end + 1..]);
        let missing = format!("{}{}", &good[..kind], &good[end + 2..]);
        assert_eq!(
            Line::parse(&mistyped),
            Err(Refusal::Write(stored::Error::KindEmpty))
        );
        assert_eq!(
            Refusal::Write(stored::Error::KindEmpty).code(),
            "kind-empty"
        );
        assert_eq!(
            Line::parse(&missing),
            Err(Refusal::Write(stored::Error::KindEmpty))
        );
        let env = good.find("\"env\":{").expect("env") + 6;
        let close = env + good[env..].find('}').expect("env end");
        let not_object = format!("{}7{}", &good[..env], &good[close + 1..]);
        assert_eq!(Line::parse(&not_object), Err(Refusal::Shape));
    }
}

#[test]
fn only_a_payload_objects_top_level_primitives_are_read() {
    let env = envelope::read(
        "env",
        &parse_json(r#"{"kind":"note","observer":"sl:a","emitted":"2026-01-01T00:00:00Z"}"#)
            .expect("JSON"),
    )
    .expect("envelope");
    let key = Key::from_text(&"0".repeat(52)).expect("seed");
    let record = |payload: &str| {
        let written = stored::write(env.clone(), vec![0; 16], parse_json(payload).expect("JSON"))
            .expect("written");
        Line::sign(&written, &key).record()
    };
    let r = record(
        r#"{"value":"v","n":3,"status":"open","done":false,"size":1.5,"deep":{"a":1},"none":null}"#,
    );
    assert_eq!(r.value, "v");
    assert_eq!(r.num("n"), Some(3_i64.into()));
    let names: Vec<_> = r.fields.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(names, ["status", "done", "size"]);
    assert_eq!(r.num("size"), None);
    // A fractional n is held and stays unread; a non-text value and a non-object payload give nothing.
    assert_eq!(record(r#"{"n":1.5}"#).num("n"), None);
    assert!(record(r#"{"n":1.5}"#).n_number.is_some());
    for payload in [r#"{"value":7,"n":"x"}"#, r#""text""#, "[1,2]", "null"] {
        let r = record(payload);
        assert_eq!(
            (r.value.as_str(), r.num("n"), r.fields.len()),
            ("", Some(0_i64.into()), 0)
        );
    }
    // The envelope a reader gets is the canonical one.
    assert!(r.about.is_empty() && r.of.is_none() && r.prev.is_empty() && r.erases.is_empty());
}
