//! Fixture operations from model/Gen/TickCheck.lean.
use super::{opened_form, sha256};
use crate::{
    fold::integer,
    reader::{list, members, object, optional, record, required, text},
};
use loops::{
    json::Json,
    record::Record,
    spec_hash::Rules,
    tick::make_tick,
    tick_check::{
        CoverClosure, MissingInputs, ReplayIssue, TickAnswer, TickCheck, TickSubject, TodayReport,
        check_tick,
    },
};

fn string(s: &str) -> Json {
    Json::Str(s.into())
}

fn missing_fields(m: MissingInputs) -> [(String, Json); 2] {
    [
        ("missing-records".into(), integer(m.records)),
        ("missing-openings".into(), integer(m.openings)),
    ]
}

fn today_json(today: TodayReport) -> Json {
    let reason = match today {
        TodayReport::Same => return string("the same result"),
        TodayReport::Another => return string("another result"),
        TodayReport::UnreadableSpec => "this binary cannot read the spec",
        TodayReport::NoncanonicalSpec => "spec is not in today's parsed form",
        TodayReport::Unavailable(ReplayIssue::IgnoredErasure) => "tick-withheld-erased",
        TodayReport::Unavailable(ReplayIssue::Missing(_)) => "missing inputs",
    };
    let mut fields = vec![("not-available".into(), string(reason))];
    if let TodayReport::Unavailable(ReplayIssue::Missing(m)) = today {
        fields.extend(missing_fields(m));
    }
    Json::Obj(fields)
}

/// The check's verdict and both reports, as written in the tick vectors.
pub fn check_json(check: TickCheck) -> Json {
    let cannot = |reason| {
        vec![
            ("answer".into(), string("cannot check")),
            ("reason".into(), string(reason)),
        ]
    };
    let mut fields = match check.answer {
        TickAnswer::NotTick(f) => vec![
            ("answer".into(), string("not a tick")),
            ("refused".into(), string(f.code())),
            ("path".into(), Json::Str(f.path)),
        ],
        TickAnswer::PayloadUnheld => cannot("the tick's payload is not held"),
        TickAnswer::UnreadableSpec => cannot("this binary cannot read the spec"),
        TickAnswer::Newer => cannot("made under newer rules"),
        TickAnswer::Older(today) => {
            let mut fields = cannot("made under older rules");
            fields.push(("today".into(), today_json(today)));
            fields
        }
        TickAnswer::Missing(m) => {
            let mut fields = cannot("missing inputs");
            fields.extend(missing_fields(m));
            fields
        }
        TickAnswer::Same => vec![("answer".into(), string("same"))],
        TickAnswer::Differs { claimed, computed } => vec![
            ("answer".into(), string("differs")),
            ("claimed-hash".into(), Json::Str(claimed)),
            ("computed-hash".into(), Json::Str(computed)),
        ],
    };
    let closure = match check.closure {
        CoverClosure::Yes => "yes",
        CoverClosure::No => "no",
        CoverClosure::Unknown => "unknown",
    };
    fields.push(("closure".into(), string(closure)));
    fields.push((
        "taken-not-covered".into(),
        check.taken_outside.map_or(Json::Null, integer),
    ));
    Json::Obj(fields)
}

fn forms(path: &str, raw: &Json) -> Result<Vec<Record>, String> {
    list(path, raw)?
        .iter()
        .enumerate()
        .map(|(i, j)| {
            let path = format!("{path}[{i}]");
            if optional(object(&path, j)?, "opening").is_some() {
                opened_form(&path, j, "opening")
            } else {
                record(&path, j)
            }
        })
        .collect()
}

pub(super) fn make_and_check(input: &Json) -> Result<Json, String> {
    let kvs = members(
        "input",
        &["op", "spec", "writer", "checker", "fresh"],
        input,
    )?;
    let spec = crate::spec::parsed(required("input", kvs, "spec")?)?;
    let writer = forms("input.writer", required("input", kvs, "writer")?)?;
    let checker = optional(kvs, "checker")
        .map(|j| forms("input.checker", j))
        .transpose()?;
    let checker = checker.as_deref().unwrap_or(&writer);
    let fresh = match optional(kvs, "fresh") {
        None => false,
        Some(Json::Bool(b)) => *b,
        _ => return Err("input.fresh: expected boolean".into()),
    };
    let made = make_tick(spec, &writer);
    let payload = made.to_json(&sha256);
    let subject = TickSubject {
        kind: "tick",
        payload: Some(&payload),
    };
    let check = check_tick(&sha256, &Rules::current(), subject, checker);
    let mut fields = vec![
        ("payload".into(), payload),
        ("check".into(), check_json(check)),
    ];
    if fresh {
        fields.push((
            "fresh".into(),
            make_tick(made.spec, checker).to_json(&sha256),
        ));
    }
    Ok(Json::Obj(fields))
}

pub(super) fn check_subject(input: &Json) -> Result<Json, String> {
    let kvs = members("input", &["op", "current", "subject", "checker"], input)?;
    let current = Rules::of_json(required("input", kvs, "current")?)
        .ok_or("input.current: invalid version")?;
    let subject = members(
        "input.subject",
        &["kind", "payload"],
        required("input", kvs, "subject")?,
    )?;
    let kind = text(
        "input.subject.kind",
        required("input.subject", subject, "kind")?,
    )?;
    let forms = forms("input.checker", required("input", kvs, "checker")?)?;
    Ok(check_json(check_tick(
        &sha256,
        &current,
        TickSubject {
            kind,
            payload: optional(subject, "payload"),
        },
        &forms,
    )))
}
