//! Drafts from the command line and from JSONL, in the design's flat shape:
//! envelope fields beside `payload`, with an optional `label` and `salt`.
use crate::{Fail, refused, usage};
use loops::{base32, batch::Draft, envelope::Env, json::Json};
use loops_place::key;

fn new_salt() -> Result<Vec<u8>, Fail> {
    key::salt().map_err(|e| Fail::System(e.to_string()))
}

/// `emit <kind> [field=text | field:=json]... ["message"]` and its flags.
/// `field=text` is always text and `field:=json` a JSON value, as `HTTPie` writes them.
pub(crate) fn from_args(
    args: &[String],
    now: &str,
    link: impl Fn(&str) -> Result<String, Fail>,
    observer: impl FnOnce(Option<String>) -> String,
) -> Result<Draft, Fail> {
    let mut env = Env {
        emitted: now.to_owned(),
        ..Env::default()
    };
    let mut given_observer = None;
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
            "--prev" => env.prev.push(link(&value()?)?),
            "--erases" => env.erases.push(link(&value()?)?),
            "--under" => env.under = Some(link(&value()?)?),
            "--occurred" => env.occurred = Some(value()?),
            "--observer" => given_observer = Some(value()?),
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
    env.observer = observer(given_observer);
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
fn text_of(line: usize, name: &str, value: &Json) -> Result<String, Fail> {
    match value {
        Json::Str(s) => Ok(s.clone()),
        _ => Err(shape(line, &format!("{name} is text"))),
    }
}
fn texts_of(line: usize, name: &str, value: &Json) -> Result<Vec<String>, Fail> {
    match value {
        Json::Arr(items) => items.iter().map(|v| text_of(line, name, v)).collect(),
        _ => Err(shape(line, &format!("{name} is an array of text"))),
    }
}

/// One draft per line: a changeset. A line names what breaks, and nothing is
/// written unless every line is taken.
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
        let mut draft = Draft {
            env: Env {
                observer: observer.to_owned(),
                emitted: now.to_owned(),
                ..Env::default()
            },
            payload: Json::Obj(Vec::new()),
            ..Draft::default()
        };
        let mut salt = None;
        for (i, (name, value)) in members.iter().enumerate() {
            if members[..i].iter().any(|(earlier, _)| earlier == name) {
                return Err(refused(format!("member-duplicate at line {line}: {name}")));
            }
            match name.as_str() {
                "label" => draft.label = Some(text_of(line, name, value)?),
                "kind" => draft.env.kind = text_of(line, name, value)?,
                "observer" => draft.env.observer = text_of(line, name, value)?,
                "emitted" => draft.env.emitted = text_of(line, name, value)?,
                "occurred" => draft.env.occurred = Some(text_of(line, name, value)?),
                "under" => draft.env.under = Some(text_of(line, name, value)?),
                "about" => draft.env.about = texts_of(line, name, value)?,
                "prev" => draft.env.prev = texts_of(line, name, value)?,
                "erases" => draft.env.erases = texts_of(line, name, value)?,
                "payload" => draft.payload = value.clone(),
                "salt" => {
                    salt = Some(
                        base32::decode(&text_of(line, name, value)?)
                            .ok_or_else(|| refused(format!("salt-not-16-bytes at line {line}")))?,
                    );
                }
                _ => {
                    return Err(refused(format!(
                        "draft-field-unknown at line {line}: {name}"
                    )));
                }
            }
        }
        draft.salt = match salt {
            Some(salt) => salt,
            None => new_salt()?,
        };
        drafts.push(draft);
    }
    Ok(drafts)
}
