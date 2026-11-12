//! Drafts from the command line and from JSONL, in the design's flat shape:
//! envelope fields beside `payload`, with an optional `label` and `salt`.
use crate::{Fail, refused, usage};
use loops::{
    base32,
    batch::Draft,
    envelope::{self, Env},
    json::Json,
};
use loops_place::key;

fn new_salt() -> Result<Vec<u8>, Fail> {
    key::salt().map_err(|e| Fail::System(e.to_string()))
}

/// `emit <kind> [field=text | field:=json]... ["message"]` and its flags.
/// `field=text` is always text and `field:=json` a JSON value, as `HTTPie` writes them.
/// `link` resolves a record id the person typed; `thing` resolves what `--of` names.
pub(crate) fn from_args(
    args: &[String],
    now: &str,
    observer: &str,
    link: impl Fn(&str) -> Result<String, Fail>,
    thing: impl Fn(&str) -> Result<String, Fail>,
) -> Result<Draft, Fail> {
    let mut env = Env {
        emitted: now.to_owned(),
        observer: observer.to_owned(),
        ..Env::default()
    };
    let mut salt = None;
    let mut kind = None;
    let mut fields: Vec<(String, Json)> = Vec::new();
    let mut message = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = || {
            rest.next()
                .cloned()
                .ok_or_else(|| usage(format!("{arg} takes a value")))
        };
        match arg.as_str() {
            "--about" => env.about.push(value()?),
            "--of" => env.of = Some(thing(&value()?)?),
            "--prev" => env.prev.push(link(&value()?)?),
            "--erases" => env.erases.push(link(&value()?)?),
            "--under" => env.under = Some(link(&value()?)?),
            "--occurred" => env.occurred = Some(value()?),
            "--observer" => env.observer = value()?,
            "--emitted" => env.emitted = value()?,
            "--salt" => {
                salt = Some(base32::decode(&value()?).ok_or_else(|| refused("salt-not-16-bytes"))?);
            }
            flag if flag.starts_with("--") => {
                return Err(usage(format!("emit does not take {flag}")));
            }
            _ if kind.is_none() => kind = Some(arg.clone()),
            _ if message.is_some() => {
                return Err(usage("the message comes last, after every field"));
            }
            field => match field.split_once('=') {
                Some((name, json)) if name.ends_with(':') && name.len() > 1 => {
                    let value =
                        loops_eval::parse_json(json).map_err(|e| usage(format!("{field}: {e}")))?;
                    fields.push((name[..name.len() - 1].to_owned(), value));
                }
                Some((name, value)) if !name.is_empty() => {
                    fields.push((name.to_owned(), Json::Str(value.to_owned())));
                }
                _ => message = Some(arg.clone()),
            },
        }
    }
    env.kind = kind.ok_or_else(|| usage("emit takes a kind"))?;
    if let Some(message) = message {
        fields.push(("text".to_owned(), Json::Str(message)));
    }
    Ok(Draft {
        label: None,
        env,
        salt: match salt {
            Some(salt) => salt,
            None => new_salt()?,
        },
        payload: Json::Obj(fields),
    })
}

fn shape(line: usize, what: &str) -> Fail {
    refused(format!("draft-shape at line {line}: {what}"))
}

/// The envelope fields a line may carry, beside `label`, `payload` and `salt`.
const ENVELOPE_FIELDS: &[&str] = &[
    "kind", "observer", "emitted", "occurred", "about", "of", "prev", "under", "erases",
];

/// One draft per line: a changeset. A line names what breaks, and nothing is
/// written unless every line is taken.
///
/// A line's own shape is the CLI's: a line that is not a JSON object or has a
/// label that is not text is `draft-shape`, a repeated member
/// `member-duplicate`, and an unknown member `draft-field-unknown`, each
/// refused at once. Its envelope fields are read by the core's reader, as
/// `Read.envOf` reads them: a field left out or of the wrong JSON type reads as
/// unreadable text, so the batch writer refuses it with that field's own code,
/// in the writer's order across the whole changeset, after label refusals.
/// `observer` and `emitted` are filled in only when the line leaves them out.
/// A salt the line gives is read as the core reads one: text that is not
/// base32hex is no bytes, refused as `salt-not-16-bytes` in that check's place.
pub(crate) fn from_jsonl(input: &str, observer: &str, now: &str) -> Result<Vec<Draft>, Fail> {
    let mut drafts = Vec::new();
    for (i, text) in input.lines().enumerate() {
        let line = i + 1;
        if text.trim().is_empty() {
            continue;
        }
        let Ok(Json::Obj(members)) = loops_eval::parse_json(text) else {
            return Err(shape(line, "a draft is a JSON object"));
        };
        let mut label = None;
        let mut payload = Json::Obj(Vec::new());
        let mut salt = None;
        let mut fields: Vec<(String, Json)> = Vec::new();
        for (i, (name, value)) in members.iter().enumerate() {
            if members[..i].iter().any(|(earlier, _)| earlier == name) {
                return Err(refused(format!("member-duplicate at line {line}: {name}")));
            }
            match name.as_str() {
                "label" => match value {
                    Json::Str(s) => label = Some(s.clone()),
                    _ => return Err(shape(line, "label is text")),
                },
                "payload" => payload = value.clone(),
                "salt" => salt = Some(envelope::read_salt(Some(value))),
                field if ENVELOPE_FIELDS.contains(&field) => {
                    fields.push((name.clone(), value.clone()));
                }
                _ => {
                    return Err(refused(format!(
                        "draft-field-unknown at line {line}: {name}"
                    )));
                }
            }
        }
        // The CLI fills in who and when only where the line leaves them out.
        for (name, value) in [("observer", observer), ("emitted", now)] {
            if !fields.iter().any(|(k, _)| k == name) {
                fields.push((name.to_owned(), Json::Str(value.to_owned())));
            }
        }
        // Every member is a known envelope field, so the core's reader only
        // reads values here; it refuses nothing a line can hold.
        let env = envelope::read(&format!("line {line}"), &Json::Obj(fields))
            .map_err(|e| shape(line, &e.to_string()))?;
        drafts.push(Draft {
            label,
            env,
            salt: match salt {
                Some(salt) => salt,
                None => new_salt()?,
            },
            payload,
        });
    }
    Ok(drafts)
}
