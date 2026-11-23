//! Drafts from the command line and from JSONL, in the design's flat shape:
//! envelope fields beside `payload`, with an optional `label` and `salt`.
use crate::{Fail, cut::Cut, link, refused, thing_ref, usage};
use loops::{
    base32,
    batch::Draft,
    envelope::{self, Env},
    json::Json,
    spec::Spec,
};
use loops_place::{
    dir::Held,
    fill::{About, Occurred, Sets, Statement},
    key,
};

fn new_salt() -> Result<Vec<u8>, Fail> {
    key::salt().map_err(|e| Fail::System(e.to_string()))
}

/// Read one positional argument: kind, payload field, or final message.
fn emit_argument(
    arg: &str,
    kind: &mut Option<String>,
    fields: &mut Vec<(String, Json)>,
    message: &mut Option<String>,
) -> Result<(), Fail> {
    if kind.is_none() {
        *kind = Some(arg.to_owned());
        return Ok(());
    }
    if message.is_some() {
        return Err(usage("the message comes last, after every field"));
    }
    match arg.split_once('=') {
        Some((name, json)) if name.ends_with(':') && name.len() > 1 => {
            let value = loops_eval::parse_json(json).map_err(|e| usage(format!("{arg}: {e}")))?;
            fields.push((name[..name.len() - 1].to_owned(), value));
        }
        Some((name, value)) if !name.is_empty() => {
            fields.push((name.to_owned(), Json::Str(value.to_owned())));
        }
        _ => *message = Some(arg.to_owned()),
    }
    Ok(())
}

/// A command-level selection; an id may name a tick literally.
#[derive(Default)]
pub(crate) struct BaseArgs {
    pub(crate) tick: Option<String>,
    pub(crate) ids: Vec<String>,
}

fn add_base(base: &mut Option<BaseArgs>, flag: &str, id: String) -> Result<(), Fail> {
    if id.is_empty() || id == "r:" || id.starts_with("--") {
        return Err(usage(format!("{flag} takes a record id or prefix")));
    }
    let base = base.get_or_insert_with(BaseArgs::default);
    if flag == "--base-id" {
        base.ids.push(id);
    } else if base.tick.is_some() {
        return Err(usage("use --base once"));
    } else {
        base.tick = Some(id);
    }
    Ok(())
}

/// The value of a flag of `emit` or `tick`. A flag may take "--" as its
/// value, and the global `--json` keeps its meaning until the terminator.
fn option_value<'a>(
    args: &mut impl Iterator<Item = &'a String>,
    flag: &str,
) -> Result<String, Fail> {
    args.find(|v| v.as_str() != "--json")
        .cloned()
        .ok_or_else(|| usage(format!("{flag} takes a value")))
}

fn next_value<'a>(args: &mut impl Iterator<Item = &'a String>, flag: &str) -> Result<String, Fail> {
    args.next()
        .cloned()
        .ok_or_else(|| usage(format!("{flag} takes a value")))
}

/// Flags after the JSONL dash, entered only for the changeset syntax.
pub(crate) fn changeset_args(args: &[String]) -> Result<(JsonlMode, Option<BaseArgs>), Fail> {
    let mut args = args.iter();
    let mut fill = false;
    let mut base = None;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--fill" if !fill => fill = true,
            "--base" | "--base-id" => {
                add_base(&mut base, flag, next_value(&mut args, flag)?)?;
            }
            _ => return Err(usage(format!("unexpected changeset option {flag}"))),
        }
    }
    if !fill && base.is_some() {
        return Err(usage("a changeset base requires --fill"));
    }
    Ok((
        if fill {
            JsonlMode::Fill
        } else {
            JsonlMode::Plain
        },
        base,
    ))
}

/// `emit <kind> [field=text | field:=json]... ["message"]` and its flags.
/// `field=text` is always text and `field:=json` a JSON value, as `HTTPie` writes them.
/// The draft comes back with whether it states its `about`: with `--about`,
/// once or more, or `--no-about` for none; without either it is copied. And
/// with whether it states its `sets`: with `--sets`, once or more,
/// `--sets-nothing` for none, or `--whole` for a whole-read emit; without
/// any of them it sets the keys it names, when its thing is held.
/// `--occurred` and `--no-occurred` each state what goes into the envelope.
/// A tick spec supplies the payload after the writer selects its holding.
pub(crate) fn from_args(
    args: &[String],
    tick_spec: Option<Spec>,
    now: &str,
    observer: &str,
    held: &Held,
) -> Result<(Statement, Option<BaseArgs>), Fail> {
    let mut base = None;
    let mut cut = Cut::default();
    let mut env = Env {
        emitted: now.to_owned(),
        observer: observer.to_owned(),
        ..Env::default()
    };
    let mut salt = None;
    let verb = if tick_spec.is_some() { "tick" } else { "emit" };
    let mut kind = tick_spec.as_ref().map(|_| "tick".to_owned());
    let mut fields: Vec<(String, Json)> = Vec::new();
    let mut message = None;
    let mut nothing = false;
    let mut whole = false;
    let mut no_about = false;
    let mut no_occurred = false;
    let mut rest = args.iter();
    let mut options = true;
    while let Some(arg) = rest.next() {
        if !options || !arg.starts_with("--") {
            if tick_spec.is_some() {
                return Err(usage("tick takes only envelope flags after the spec"));
            }
            emit_argument(arg, &mut kind, &mut fields, &mut message)?;
            continue;
        }
        let mut value = || option_value(&mut rest, arg);
        match arg.as_str() {
            "--" => options = false,
            "--json" => {}
            "--as-of" | "--cover" if tick_spec.is_some() => cut.select(arg, value()?)?,
            "--all" | "--slice" if tick_spec.is_some() => cut.mode(arg)?,
            "--about" => env.about.push(value()?),
            "--base" | "--base-id" => add_base(&mut base, arg, value()?)?,
            "--no-about" => no_about = true,
            "--of" => env.of = Some(thing_ref(held, &value()?)?),
            "--prev" => env.prev.push(link(held, &value()?)?),
            "--erases" => env.erases.push(link(held, &value()?)?),
            "--under" => env.under = Some(link(held, &value()?)?),
            // `--sets` names one key the emit sets; `--sets-nothing` states an
            // empty `sets`, an emit that sets no key; `--whole` states no
            // `sets`, an emit read whole. Without any of them an emit of a
            // held thing sets the keys it names (`fill::sets`).
            "--sets" => env.sets.get_or_insert_with(Vec::new).push(value()?),
            "--sets-nothing" => {
                nothing = true;
                env.sets.get_or_insert_with(Vec::new);
            }
            "--whole" => whole = true,
            "--occurred" => env.occurred = Some(value()?),
            "--no-occurred" => no_occurred = true,
            "--observer" => env.observer = value()?,
            "--emitted" => env.emitted = value()?,
            "--salt" => {
                salt = Some(base32::decode(&value()?).ok_or_else(|| refused("salt-not-16-bytes"))?);
            }
            flag => return Err(usage(format!("{verb} does not take {flag}"))),
        }
    }
    if nothing && env.sets.as_ref().is_some_and(|keys| !keys.is_empty()) {
        return Err(usage("--sets-nothing takes no --sets beside it"));
    }
    if whole && env.sets.is_some() {
        return Err(usage("--whole takes no --sets or --sets-nothing beside it"));
    }
    if no_about && !env.about.is_empty() {
        return Err(usage("--no-about takes no --about beside it"));
    }
    let occurred = match (no_occurred, env.occurred.is_some()) {
        (true, true) => return Err(usage("--no-occurred takes no --occurred beside it")),
        (false, false) => Occurred::Copied,
        _ => Occurred::Stated,
    };
    let about = if no_about || !env.about.is_empty() {
        About::Stated
    } else {
        About::Copied
    };
    env.kind = kind.ok_or_else(|| usage("emit takes a kind"))?;
    if let Some(message) = message {
        fields.push(("text".to_owned(), Json::Str(message)));
    }
    let sets = if whole || env.sets.is_some() {
        Sets::Stated
    } else {
        Sets::Named
    };
    let draft = Draft {
        label: None,
        env,
        salt: salt.map_or_else(new_salt, Ok)?,
        payload: match tick_spec {
            Some(spec) => cut.payload(held, spec)?,
            None => Json::Obj(fields),
        },
    };
    let statement = Statement {
        draft,
        about,
        occurred,
        sets,
    };
    Ok((statement, base))
}

fn shape(line: usize, what: &str) -> Fail {
    refused(format!("draft-shape at line {line}: {what}"))
}

/// The envelope fields a line may carry, beside `label`, `payload` and `salt`.
const ENVELOPE_FIELDS: &[&str] = &[
    "kind", "observer", "emitted", "occurred", "about", "of", "prev", "under", "erases", "sets",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsonlMode {
    Plain,
    Fill,
}

/// One draft per line: a changeset. A line names what breaks, and nothing is
/// written unless every line is taken.
///
/// A line's own shape is the CLI's: a line that is not a JSON object or has a
/// label that is not text is `draft-shape`, a repeated member
/// `member-duplicate`, and an unknown member `draft-field-unknown`, each
/// refused at once. Its envelope fields are read by the core's reader, as
/// `Read.envOf` reads them: a field left out or of the wrong JSON type reads as
/// unreadable text, so the batch writer refuses it with that field's own code,
/// after label refusals (across the whole changeset in plain mode). In fill mode
/// only, null `occurred` or `sets` states the absence of that optional field.
/// `observer` and `emitted` are filled in only when the line leaves them out.
/// A salt the line gives is read as the core reads one: text that is not
/// base32hex is no bytes, refused as `salt-not-16-bytes` in that check's place.
pub(crate) fn from_jsonl(
    input: &str,
    observer: &str,
    now: &str,
    mode: JsonlMode,
) -> Result<Vec<Statement>, Fail> {
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
        let mut about = About::Copied;
        let mut occurred = Occurred::Copied;
        let mut sets = Sets::Named;
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
                    match field {
                        "about" => about = About::Stated,
                        "occurred" => occurred = Occurred::Stated,
                        "sets" => sets = Sets::Stated,
                        _ => {}
                    }
                    if mode == JsonlMode::Fill
                        && *value == Json::Null
                        && matches!(field, "occurred" | "sets")
                    {
                        continue;
                    }
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
        let draft = Draft {
            label,
            env,
            salt: salt.map_or_else(new_salt, Ok)?,
            payload,
        };
        drafts.push(Statement {
            draft,
            about,
            occurred,
            sets,
        });
    }
    Ok(drafts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_presence_survives_decoding_for_the_filler() {
        for mode in [JsonlMode::Fill, JsonlMode::Plain] {
            let rows = from_jsonl(
                r#"{}
{"about":[],"occurred":null,"prev":[],"sets":[]}
{"sets":null}
{"about":["p:x"],"occurred":"2009","sets":["k"],"prev":["@x"]}
{"about":null,"occurred":7,"prev":null,"sets":7}"#,
                "k:test",
                "now",
                mode,
            )
            .ok()
            .expect("shape passes");
            let [missing, empty, whole, stated, bad] = rows.as_slice() else {
                panic!("five rows")
            };
            assert_eq!(
                (missing.about, missing.occurred, missing.sets),
                (About::Copied, Occurred::Copied, Sets::Named)
            );
            assert_eq!(
                (empty.about, empty.occurred, empty.sets),
                (About::Stated, Occurred::Stated, Sets::Stated)
            );
            assert_eq!(missing.draft.env.about, empty.draft.env.about);
            assert_eq!(missing.draft.env.prev, empty.draft.env.prev);
            assert_eq!(missing.draft.env.sets, None);
            assert_eq!(empty.draft.env.sets, Some(vec![]));
            assert_eq!(whole.sets, Sets::Stated);
            if mode == JsonlMode::Fill {
                assert_eq!(empty.draft.env.occurred, None);
                assert_eq!(whole.draft.env.sets, None);
            } else {
                assert_eq!(empty.draft.env.occurred.as_deref(), Some(""));
                assert_eq!(whole.draft.env.sets, Some(vec![String::new()]));
            }
            assert_eq!(stated.draft.env.about, ["p:x"]);
            assert_eq!(stated.draft.env.prev, ["@x"]);
            assert_eq!(stated.draft.env.occurred.as_deref(), Some("2009"));
            assert_eq!(stated.draft.env.sets, Some(vec!["k".into()]));
            for row in [stated, bad] {
                assert_eq!(
                    (row.about, row.occurred, row.sets),
                    (About::Stated, Occurred::Stated, Sets::Stated)
                );
            }
            assert_eq!(bad.draft.env.about, [""]);
            assert_eq!(bad.draft.env.prev, [""]);
            assert_eq!(bad.draft.env.occurred.as_deref(), Some(""));
            assert_eq!(bad.draft.env.sets, Some(vec![String::new()]));
        }
    }
}
