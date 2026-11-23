//! Map one arrival against the history of this destination only.
use super::source::{Fact, Fold, Map};
use loops::{
    batch::Draft,
    envelope::{self, Env},
    json::{Json, canon_set},
    stored,
};
use loops_place::{clock, key::Key, line::hash};
use std::{collections::HashMap, fmt};

/// Use the exact binary64 value. Fixed-point formatting rounds directly to
/// six decimal places, including ties to even. Multiplying by 1e6 in f64
/// first can round twice at contemporary epoch values.
pub(super) fn occurred(bits: u64) -> Option<String> {
    let seconds = f64::from_bits(bits);
    if !(-62_167_219_200.0..253_402_300_800.0).contains(&seconds) {
        return None;
    }
    let rounded = format!("{seconds:.6}");
    let (whole, fraction) = rounded.trim_start_matches('-').split_once('.')?;
    let magnitude = whole.parse::<i64>().ok()? * 1_000_000 + fraction.parse::<i64>().ok()?;
    let micros = if seconds.is_sign_negative() {
        -magnitude
    } else {
        magnitude
    };
    let seconds = micros.div_euclid(1_000_000);
    let nanos = u32::try_from(micros.rem_euclid(1_000_000)).ok()? * 1000;
    if seconds >= 0 {
        return clock::instant(u64::try_from(seconds).ok()?, nanos);
    }
    // Five Gregorian 400-year cycles preserve month, day, and leap days.
    // Shift into the domain of the place's UTC formatter, then undo the shift.
    let shifted = u64::try_from(seconds + 5 * 146_097 * 86_400).ok()?;
    let text = clock::instant(shifted, nanos)?;
    let year = text[..4].parse::<u32>().ok()?.checked_sub(2000)?;
    Some(format!("{year:04}{}", &text[4..]))
}

#[derive(Debug)]
pub(super) struct Prepared {
    pub id: String,
    pub draft: Draft,
    pub missing_key: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Unnamed {
    Omit,
    Roots,
}

#[derive(Debug)]
pub(super) enum Refusal {
    Source(String),
    Map(&'static str),
    Writer(stored::Error),
}

impl Refusal {
    pub(super) fn code(&self) -> &'static str {
        match self {
            Self::Source(_) => "shadow-source-invalid",
            Self::Map(_) => "shadow-map-invalid",
            Self::Writer(error) => error.code(),
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(reason) => write!(f, "{}: {reason}", self.code()),
            Self::Map(reason) => write!(f, "{}: {reason}", self.code()),
            Self::Writer(error) => error.fmt(f),
        }
    }
}

#[derive(Debug)]
pub(super) enum Row {
    Included(Box<Prepared>),
    Omitted { defect: Option<Refusal> },
}

/// Every ordinary keyed revision in an importer-owned history names all live
/// records. Consequently there is one head and at most one setter per key.
#[derive(Debug)]
struct Live {
    whole: String,
    head: String,
    setters: HashMap<String, String>,
}

impl Live {
    fn predecessors(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.setters.values().cloned().collect();
        ids.extend([self.whole.clone(), self.head.clone()]);
        canon_set(&ids)
    }
}

#[derive(Debug, Default)]
pub(super) struct History {
    things: HashMap<String, Live>,
}

fn contents(fact: &Fact) -> Result<(Json, String), Refusal> {
    match fact.kind.as_str() {
        "" => return Err(Refusal::Writer(stored::Error::KindEmpty)),
        "tick" => return Err(Refusal::Source("legacy fact kind tick is reserved".into())),
        _ => {}
    }
    let payload = fact
        .parsed_payload()
        .map_err(|e| Refusal::Source(e.to_string()))?;
    if !matches!(payload, Json::Obj(_)) {
        return Err(Refusal::Source("payload must be an object".into()));
    }
    if !payload.keys_distinct() {
        return Err(Refusal::Writer(stored::Error::MemberDuplicate));
    }
    if !payload.in_range() {
        return Err(Refusal::Writer(stored::Error::NumberOutOfRange));
    }
    let time = occurred(fact.ts_bits).ok_or(Refusal::Writer(stored::Error::OccurredNotSpan))?;
    Ok((payload, time))
}

impl History {
    // C supplies the group reservation for this old id. Neither clock nor
    // randomness is consulted while preparing the draft or replaying it.
    pub(super) fn prepare(
        &mut self,
        map: &Map,
        unnamed: Unnamed,
        fact: &Fact,
        key: &Key,
        salt: &[u8],
        emitted: &str,
    ) -> Result<Row, Refusal> {
        let fold = map.kinds.get(&fact.kind);
        let contents = contents(fact);
        if fold.is_none() && unnamed == Unnamed::Omit {
            return Ok(Row::Omitted {
                defect: contents.err(),
            });
        }
        let (payload, time) = contents?;
        let Json::Obj(members) = &payload else {
            unreachable!("validated object")
        };
        let of = if let Some(Fold::By(field)) = fold {
            if fact.kind == "r" || fact.kind.contains(':') {
                return Err(Refusal::Map(
                    "keyed kind must be a single reference prefix other than r",
                ));
            }
            if !envelope::typed_ref(&format!("{}:x", fact.kind)) {
                return Err(Refusal::Writer(stored::Error::OfNotTyped));
            }
            match members.iter().find(|(name, _)| name == field) {
                Some((_, Json::Str(value))) if value.is_empty() => {
                    return Err(Refusal::Writer(stored::Error::OfNotTyped));
                }
                Some((_, Json::Str(value))) => Some(format!("{}:{value}", fact.kind)),
                Some(_) => {
                    return Err(Refusal::Source(format!(
                        "key field {field:?} must hold text"
                    )));
                }
                None => None,
            }
        } else {
            None
        };
        let missing_key = matches!(fold, Some(Fold::By(_))) && of.is_none();
        let live = of.as_ref().and_then(|thing| self.things.get(thing));
        let env = Env {
            kind: fact.kind.clone(),
            observer: key.public(),
            of,
            occurred: Some(time),
            emitted: emitted.into(),
            about: if fact.observer.is_empty() {
                Vec::new()
            } else {
                vec![format!("observer:{}", fact.observer)]
            },
            sets: live
                .map(|_| canon_set(&members.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>())),
            prev: live.map(Live::predecessors).unwrap_or_default(),
            ..Env::default()
        };
        let draft = Draft {
            label: None,
            env,
            salt: salt.to_vec(),
            payload,
        };
        // Use the writer's field checks before canonicalizing or advancing state.
        let written = stored::write(draft.env.clone(), draft.salt.clone(), draft.payload.clone())
            .map_err(Refusal::Writer)?;
        let id = stored::id_of(&hash, written.env(), &draft.commit(&hash));
        if let Some(thing) = &draft.env.of {
            let state = self.things.entry(thing.clone()).or_insert_with(|| Live {
                whole: id.clone(),
                head: id.clone(),
                setters: HashMap::new(),
            });
            state.head.clone_from(&id);
            for field in draft.env.sets.iter().flatten() {
                state.setters.insert(field.clone(), id.clone());
            }
        }
        Ok(Row::Included(Box::new(Prepared {
            id,
            draft,
            missing_key,
        })))
    }
}
