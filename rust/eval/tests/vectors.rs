//! Every family is registered. Tick operations are counted as run or pending.
mod common;
use common::{expected, json_files, root};
use loops::{json::Json, occurred, time::Moment};
use serde_json::{Value, json};
use std::{fs, path::Path};

const FAMILIES: &[(&str, usize)] = &[
    ("record", 197),
    ("payload", 18),
    ("occurred", 9),
    ("byte-text", 5),
    ("fold", 50),
    ("spec", 220),
    ("realm", 16),
    ("scenario", 7),
    ("lens", 21),
    ("batch", 83),
    ("standard", 12),
    ("result", 23),
    ("tick", 11),
];

fn member<'a>(j: &'a Json, key: &str) -> &'a Json {
    let Json::Obj(kvs) = j else {
        panic!("object expected")
    };
    &kvs.iter().find(|(k, _)| k == key).expect("member").1
}
// Gen.OccurredVectors.utc, independent of the evaluator's seven fuzz families.
fn utc(moment: &Moment) -> String {
    let seconds = moment.seconds();
    let shifted_day = seconds.div_euclid(86400) + 719_468;
    let era = shifted_day.div_euclid(146_097);
    let doe = shifted_day - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year_number = yoe + era * 400 + i64::from(month <= 2);
    let year = if year_number < 0 {
        format!("-{:04}", -year_number)
    } else {
        format!("{year_number:04}")
    };
    let day_seconds = seconds.rem_euclid(86400);
    let frac = if moment.fraction().is_empty() {
        String::new()
    } else {
        format!(
            ".{}",
            moment
                .fraction()
                .iter()
                .map(u8::to_string)
                .collect::<String>()
        )
    };
    format!(
        "{year}-{month:02}-{day:02}T{:02}:{:02}:{:02}{frac}Z",
        day_seconds / 3600,
        day_seconds / 60 % 60,
        day_seconds % 60
    )
}
fn occurred_answer(vector: &Value) -> Value {
    let texts: Vec<_> = vector["input"]["occurred"]
        .as_array()
        .expect("occurred")
        .iter()
        .map(|v| v.as_str().expect("text"))
        .collect();
    if vector["vector"] == "08-order" {
        let mut spans: Vec<_> = texts
            .into_iter()
            .map(|s| (s, occurred::occurred_span(s).expect("span")))
            .collect();
        spans.sort_by(|a, b| a.1.cmp(&b.1));
        json!({"oldest-first":spans.into_iter().map(|(s,_)| s).collect::<Vec<_>>()})
    } else {
        json!({"spans":texts.iter().map(|s| match occurred::occurred_span(s) {
            None => json!({"refused":"occurred-not-span"}),
            Some(span) => json!({"start":utc(span.start()),"end":span.stop().map(utc)}),
        }).collect::<Vec<_>>()})
    }
}

/// The inputs that carry a number past binary64's range (D3). The public
/// strict decoder refuses them; `evaluate` decodes each as its layer reads it.
const OVERFLOWING: [&str; 8] = [
    "batch/23-overflow-out-of-range",
    "batch/24-overflow-after-envelope",
    "batch/25-overflow-before-prev",
    "record/d3-overflow-value",
    "record/d3-overflow-value-negative",
    "spec/d3-overflow-argument",
    "spec/d3-overflow-bound",
    "spec/d3-overflow-limit",
];

/// The member names of every object, in order, at every depth: what the
/// canonical comparison, which sorts members, cannot see.
fn member_order(j: &Json) -> Vec<(String, Vec<String>)> {
    fn walk(j: &Json, at: &str, out: &mut Vec<(String, Vec<String>)>) {
        match j {
            Json::Obj(kvs) => {
                out.push((at.into(), kvs.iter().map(|(k, _)| k.clone()).collect()));
                for (k, v) in kvs {
                    walk(v, &format!("{at}.{k}"), out);
                }
            }
            Json::Arr(xs) => {
                for (i, x) in xs.iter().enumerate() {
                    walk(x, &format!("{at}[{i}]"), out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(j, "", &mut out);
    out
}

/// Every answer `answers` lists for a vector the strict decoder reads, against
/// its `expect`. False when the strict decoder refuses the vector.
fn every_listed_answer(source: &str, file: &Path, want: &Json, failures: &mut Vec<String>) -> bool {
    let Ok(raw) = loops_eval::parse_json(source) else {
        return false;
    };
    let Json::Str(layer) = member(&raw, "layer") else {
        panic!("layer")
    };
    if let Ok(answers) = loops_eval::answers(layer, member(&raw, "input")) {
        assert!(!answers.is_empty(), "{}", file.display());
        for (i, answer) in answers.iter().enumerate() {
            if answer.canon() != want.canon() {
                failures.push(format!("{}: order/variant {i} differs", file.display()));
            }
            // A spec's slots, and the groups and things in them, in the model's member order.
            if layer == "spec/run" && member_order(answer) != member_order(member(&raw, "expect")) {
                failures.push(format!(
                    "{}: order {i} writes members in another order",
                    file.display()
                ));
            }
        }
    }
    true
}

fn assert_family_count(name: &str, files: usize) {
    let (_, count) = FAMILIES
        .iter()
        .find(|(family, _)| *family == name)
        .expect("registered family");
    assert_eq!(files, *count, "{name}: update exact registry after review");
}

fn check_standard(path: &Path, failures: &mut Vec<String>) -> usize {
    // Each input and output is consumed once, as six byte-for-byte pairs.
    let inputs = json_files(&path.join("jcs/input"));
    let outputs = json_files(&path.join("jcs/output"));
    assert_eq!((inputs.len(), outputs.len()), (6, 6));
    let mut checked = 0;
    for input in inputs {
        let output = path
            .join("jcs/output")
            .join(input.file_name().expect("name"));
        let got = loops_eval::parse_json(&fs::read_to_string(&input).expect("JCS input"))
            .expect("RFC8259")
            .canon();
        let want = fs::read(&output).expect("JCS output");
        if got.as_bytes() != want {
            failures.push(format!("{}: JCS bytes differ", input.display()));
        }
        checked += 2;
    }
    checked
}

#[test]
fn every_registered_json_and_every_arrival_order() {
    let root = root();
    let mut checked = 0;
    let mut pending = 0;
    let mut seen = Vec::new();
    let mut failures = Vec::new();
    let mut overflowing = Vec::new();
    for entry in fs::read_dir(&root).expect("vectors") {
        let path = entry.expect("family").path();
        if !path.is_dir() {
            assert_ne!(
                path.extension().and_then(|x| x.to_str()),
                Some("json"),
                "unregistered root JSON"
            );
            continue;
        }
        let family = path
            .file_name()
            .expect("name")
            .to_str()
            .expect("UTF8")
            .to_owned();
        let before = checked;
        if family == "standard" {
            checked += check_standard(&path, &mut failures);
        } else {
            for file in json_files(&path) {
                let source = fs::read_to_string(&file).expect("vector");
                let vector: Value = serde_json::from_str(&source).expect("vector JSON");
                if family == "tick" {
                    assert_eq!(vector["layer"], "tick", "{file:?}");
                    let op = vector["input"]["op"].as_str().expect("tick input.op");
                    if !loops_eval::supports_tick_op(op) {
                        pending += 1;
                        continue;
                    }
                }
                let want = expected(vector.get("expect").expect("fixture expect"));
                if family == "occurred" {
                    assert_eq!(
                        expected(&occurred_answer(&vector)).canon(),
                        want.canon(),
                        "{file:?}"
                    );
                } else {
                    // The actual evaluator gets the original text, never a Value
                    // rewrite that could lose input ordering/repeated members.
                    match loops_eval::evaluate(&source) {
                        Ok(got) if got.canon() == want.canon() => (),
                        Ok(got) => failures.push(format!(
                            "{file:?}: got {} expected {}",
                            got.canon(),
                            want.canon()
                        )),
                        Err(error) => failures.push(format!("{file:?}: {error}")),
                    }
                    if !every_listed_answer(&source, &file, &want, &mut failures) {
                        overflowing.push(format!(
                            "{family}/{}",
                            file.file_stem().expect("stem").to_str().expect("UTF8")
                        ));
                    }
                }
                checked += 1;
            }
        }
        if family == "tick" && pending != 0 {
            println!("tick: {pending} pending");
        }
        assert_family_count(&family, checked - before);
        seen.push(family);
    }
    for (family, _) in FAMILIES {
        assert!(seen.iter().any(|s| s == family), "missing {family}");
    }
    assert_eq!(pending, 0, "tick: update pending count after review");
    assert_eq!(checked, 672);
    overflowing.sort();
    assert_eq!(
        overflowing, OVERFLOWING,
        "only these inputs hold a number the strict decoder refuses"
    );
    assert_eq!(
        checked + pending,
        json_files(&root).len(),
        "unconsumed JSON"
    );
    assert_eq!(
        failures,
        Vec::<String>::new(),
        "all registered files executed"
    );
}

#[test]
fn expectation_integers_are_not_rounded_like_input_numbers() {
    let raw = "18014398509481981";
    let v: Value = serde_json::from_str(raw).expect("output JSON");
    assert_eq!(expected(&v).canon(), raw);
    assert_ne!(
        loops_eval::parse_json(raw).expect("input JSON").canon(),
        raw
    );
}
