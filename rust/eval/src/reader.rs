//! Gen.Read.record (model/Gen/Read.lean): shape checks, no writer-only validation.
use loops::{
    base32, envelope,
    json::Json,
    occurred::{Claim, Stamp},
    record::{Digest, Held, Record, Shown, id_text, payload_fields},
};

/// Gen.Read.object, a fixture object with distinct member names.
pub(super) fn object<'a>(path: &str, j: &'a Json) -> Result<&'a [(String, Json)], String> {
    let Json::Obj(kvs) = j else {
        return Err(format!("{path}: expected object"));
    };
    // Gen.Read.object checks duplicates before members? checks unknown keys.
    for (i, (k, _)) in kvs.iter().enumerate() {
        if kvs[..i].iter().any(|(earlier, _)| earlier == k) {
            return Err(format!("{path}: duplicate member {k}"));
        }
    }
    Ok(kvs)
}
/// Gen.Read.members? (model/Gen/Read.lean), duplicates before unknown members.
pub(super) fn members<'a>(
    path: &str,
    allowed: &[&str],
    j: &'a Json,
) -> Result<&'a [(String, Json)], String> {
    let kvs = object(path, j)?;
    for (k, _) in kvs {
        if !allowed.contains(&k.as_str()) {
            return Err(format!("{path}: unknown member {k}"));
        }
    }
    Ok(kvs)
}
/// Gen.Read.req (model/Gen/Read.lean), no hidden fixture defaults.
pub(super) fn required<'a>(
    path: &str,
    kvs: &'a [(String, Json)],
    name: &str,
) -> Result<&'a Json, String> {
    optional(kvs, name).ok_or_else(|| format!("{path}: missing {name}"))
}
pub(super) fn optional<'a>(kvs: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
    kvs.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}
/// Gen.Read.list (model/Gen/Read.lean), strict array shape.
pub(super) fn list<'a>(path: &str, j: &'a Json) -> Result<&'a [Json], String> {
    match j {
        Json::Arr(xs) => Ok(xs),
        _ => Err(format!("{path}: expected array")),
    }
}
pub(super) fn text<'a>(path: &str, j: &'a Json) -> Result<&'a str, String> {
    match j {
        Json::Str(s) => Ok(s),
        _ => Err(format!("{path}: expected text")),
    }
}
pub(super) fn texts(path: &str, j: &Json) -> Result<Vec<String>, String> {
    list(path, j)?
        .iter()
        .enumerate()
        .map(|(i, j)| text(&format!("{path}[{i}]"), j).map(str::to_owned))
        .collect()
}
/// Gen.Read.unbase32 (model/Gen/Read.lean), canonical lowercase byte spelling.
fn unbase32(path: &str, s: &str) -> Result<Vec<u8>, String> {
    base32::decode(s).ok_or_else(|| format!("{path}: noncanonical base32hex"))
}
/// Gen.Read.bytes (model/Gen/Read.lean), strict text and canonical byte decoding.
pub(super) fn bytes(path: &str, j: &Json) -> Result<Vec<u8>, String> {
    unbase32(path, text(path, j)?)
}
pub(super) fn reader_id(path: &str, j: &Json) -> Result<Digest, String> {
    let s = text(path, j)?;
    if !envelope::record_id(s) {
        return Err(format!("{path}: expected record id"));
    }
    let bytes = unbase32(path, &s[2..])?;
    let digest = Digest::try_from(bytes.as_slice()).map_err(|e| format!("{path}: {e}"))?;
    if id_text(&digest) != s {
        return Err(format!("{path}: id does not round trip"));
    }
    Ok(digest)
}
fn ids(path: &str, j: &Json) -> Result<Vec<Digest>, String> {
    list(path, j)?
        .iter()
        .enumerate()
        .map(|(i, j)| reader_id(&format!("{path}[{i}]"), j))
        .collect()
}
pub(super) fn shown(path: &str, j: &Json) -> Result<Shown, String> {
    match text(path, j)? {
        "here" => Ok(Shown::Here),
        "withheld" => Ok(Shown::Withheld),
        "erased" => Ok(Shown::Erased),
        _ => Err(format!("{path}: unknown shown")),
    }
}
fn fields(path: &str, j: &Json) -> Result<Vec<(String, Held)>, String> {
    if !matches!(j, Json::Obj(_)) {
        return Err(format!("{path}: expected fields object"));
    }
    if !j.keys_distinct() {
        return Err(format!("{path}: repeated field"));
    }
    Ok(payload_fields(j))
}

/// Gen.Read.record (model/Gen/Read.lean), a strict reader form, not writer validation.
pub(super) fn record(path: &str, j: &Json) -> Result<Record, String> {
    let kvs = members(
        path,
        &[
            "id", "kind", "observer", "about", "of", "prev", "under", "emitted", "occurred",
            "erases", "sets", "fields", "shown",
        ],
        j,
    )?;
    let req = |k| required(path, kvs, k);
    let p = |k| format!("{path}.{k}");
    let id = reader_id(&p("id"), req("id")?)?;
    let kind = text(&p("kind"), req("kind")?)?.into();
    let observer = text(&p("observer"), req("observer")?)?.into();
    let about = texts(&p("about"), req("about")?)?;
    for (i, reference) in about.iter().enumerate() {
        if !envelope::typed_ref(reference) {
            return Err(format!("{}[{i}]: expected a typed reference", p("about")));
        }
    }
    let of = optional(kvs, "of")
        .map(|j| {
            let s = text(&p("of"), j)?;
            if envelope::typed_ref(s) {
                Ok(s.to_owned())
            } else {
                Err(format!("{}: expected a typed reference", p("of")))
            }
        })
        .transpose()?;
    let prev = ids(&p("prev"), req("prev")?)?;
    let under = optional(kvs, "under")
        .map(|j| reader_id(&p("under"), j))
        .transpose()?;
    let emitted = Stamp::of(text(&p("emitted"), req("emitted")?)?)
        .ok_or_else(|| format!("{}: invalid stamp", p("emitted")))?;
    let occurred = optional(kvs, "occurred")
        .map(|j| {
            Claim::of(text(&p("occurred"), j)?)
                .ok_or_else(|| format!("{}: invalid claim", p("occurred")))
        })
        .transpose()?;
    let erases = optional(kvs, "erases")
        .map(|j| ids(&p("erases"), j))
        .transpose()?
        .unwrap_or_default();
    let sets = optional(kvs, "sets")
        .map(|j| {
            let keys = texts(&p("sets"), j)?;
            match keys.iter().position(String::is_empty) {
                Some(i) => Err(format!("{}[{i}]: a key name is not empty", p("sets"))),
                None => Ok(keys),
            }
        })
        .transpose()?;
    let shown = optional(kvs, "shown")
        .map(|j| shown(&p("shown"), j))
        .transpose()?
        .unwrap_or_default();
    let fields = match optional(kvs, "fields") {
        None => Vec::new(),
        Some(j) if shown == Shown::Here => fields(&p("fields"), j)?,
        Some(_) => return Err(format!("{path}: hidden payload carries fields")),
    };
    Ok(Record {
        id,
        kind,
        observer,
        about,
        of,
        prev,
        under,
        emitted,
        occurred,
        erases,
        sets,
        shown,
        fields,
    })
}

#[cfg(test)]
mod tests {
    use super::{reader_id, record};
    use loops::{
        base32,
        json::Json,
        record::{Digest, id_text},
    };

    #[test]
    fn digest_text_round_trips_every_bit_and_all_reader_links() {
        let mut digests = vec![Digest([0; 32]), Digest([255; 32])];
        for index in 0..32 {
            for bit in 0..8 {
                let mut bytes = [0; 32];
                bytes[index] = 1 << bit;
                digests.push(Digest(bytes));
            }
        }
        for digest in digests {
            let text = Json::Str(id_text(&digest));
            assert_eq!(reader_id("id", &text).expect("digest"), digest);
            let raw = Json::Obj(vec![
                ("id".into(), text.clone()),
                ("kind".into(), Json::Str("k".into())),
                ("observer".into(), Json::Str("o:x".into())),
                ("about".into(), Json::Arr(vec![])),
                ("prev".into(), Json::Arr(vec![text.clone(), text.clone()])),
                ("under".into(), text.clone()),
                ("erases".into(), Json::Arr(vec![text])),
                ("emitted".into(), Json::Str("1970-01-01T00:00:00Z".into())),
            ]);
            let r = record("record", &raw).expect("reader form");
            assert_eq!(r.id, digest);
            assert_eq!(r.prev, vec![digest, digest]);
            assert_eq!(r.under, Some(digest));
            assert_eq!(r.erases, vec![digest]);
        }
    }

    #[test]
    fn typed_about_and_numeric_fields_keep_reader_boundaries() {
        let raw = r#"{"id":"r:0000000000000000000000000000000000000000000000000000","kind":"note","observer":"sl:a","about":["thing:a"],"prev":[],"emitted":"2026-01-01T00:00:00Z","fields":{"n":1.0,"whole":2e0,"fraction":1.5,"unsafe":9007199254740992}}"#;
        let input = crate::parse(raw);
        let r = record("record", &input).expect("reader shape");
        assert_eq!(r.num("n"), Some(1_i64.into()));
        assert_eq!(r.num("whole"), Some(2_i64.into()));
        assert_eq!(r.num("fraction"), None);
        assert_eq!(r.num("unsafe"), None);
        let fractional = record(
            "record",
            &crate::parse(&raw.replace("\"n\":1.0", "\"n\":1.5")),
        )
        .expect("fractional payload");
        assert!(matches!(
            fractional.get("n"),
            Some(loops::record::Held::One(loops::record::Val::Number(_)))
        ));
        assert_eq!(fractional.num("n"), None);
        let bad = crate::parse(&raw.replace("thing:a", "not typed"));
        assert!(record("record", &bad).is_err());
        let of = record(
            "record",
            &crate::parse(&raw.replace(r#""prev""#, r#""of":"thing:b","prev""#)),
        )
        .expect("typed of");
        assert_eq!(of.of.as_deref(), Some("thing:b"));
        assert_eq!(r.of, None);
        for bad in [r#""of":"not typed","#, r#""of":7,"#, r#""of":null,"#] {
            let input = crate::parse(&raw.replace(r#""prev""#, &format!(r#"{bad}"prev""#)));
            assert!(record("record", &input).is_err(), "{bad}");
        }
    }

    #[test]
    fn reader_refuses_wrong_digest_lengths_and_nonzero_padding() {
        for len in [0, 1, 31, 33, 64] {
            let raw = Json::Str(format!("r:{}", base32::base32(&vec![0; len])));
            assert!(reader_id("id", &raw).is_err(), "length {len}");
        }
        let raw = Json::Str(format!("r:{}b", "0".repeat(51)));
        assert!(reader_id("id", &raw).is_err());
    }
}
