//! Human-readable views of inventory, things and spec results.
mod fold;
mod json_syntax;
pub(crate) use fold::fold;

use crate::{
    Fail, usage,
    values::{Field, Fields, thing_about},
};
use loops::{
    json::{Json, quote},
    live::LiveState,
    record::{Record, Shown, id_text},
    time::{Moment, instant},
};
use loops_place::{clock, dir::Held};
use std::{env, fmt::Write};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Json,
    Plain,
}

fn unicode_escape(out: &mut String, c: char) {
    for unit in c.encode_utf16(&mut [0; 2]) {
        write!(out, "\\u{unit:04x}").expect("write string");
    }
}

/// JCS escapes ASCII controls. Also neutralize DEL, C1, line separators
/// and invisible formatting controls that could change the terminal layout.
fn visible(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_control() || (!c.is_ascii() && c.escape_debug().count() > 1) {
            unicode_escape(&mut out, c);
        } else {
            out.push(c);
        }
    }
    out
}

pub(crate) fn label(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".into();
    }
    let quoted = quote(s);
    visible(&quoted[1..quoted.len() - 1])
}

fn text(s: &str) -> String {
    let conflict = s
        .strip_prefix("conflict (")
        .and_then(|n| n.strip_suffix(')'))
        .is_some_and(|n| {
            !n.is_empty()
                && (n == "0" || !n.starts_with('0'))
                && n.bytes().all(|b| b.is_ascii_digit())
        });
    if s.is_empty()
        || s.trim() != s
        || matches!(s, "—" | "unknown" | "null" | "true" | "false")
        || conflict
        || json_syntax::number_or_container(s)
    {
        visible(&quote(s))
    } else {
        label(s)
    }
}

fn value(v: &Json) -> String {
    match v {
        Json::Str(s) => text(s),
        _ => visible(&v.canon()),
    }
}

pub(crate) fn field(f: &Field) -> String {
    match f {
        Field::Value(v) => value(v),
        Field::Unset => "—".into(),
        Field::Unknown(_) => "unknown".into(),
        Field::Conflict(rs) => format!("conflict ({})", rs.len()),
    }
}

/// Use single-column spellings where padding matters, without assuming
/// terminal-specific widths for combining marks, wide glyphs or emoji.
pub(crate) fn table(header: Vec<String>, rows: impl Iterator<Item = Vec<String>>) -> String {
    let mut widths = vec![0; header.len() - 1];
    let mut rows: Vec<_> = std::iter::once(header).chain(rows).collect();
    for row in &mut rows {
        for (w, cell) in widths.iter_mut().zip(row) {
            if !cell.is_ascii() && cell != "—" {
                let mut escaped = String::new();
                for c in cell.chars() {
                    if c.is_ascii() || c == '—' {
                        escaped.push(c);
                    } else {
                        unicode_escape(&mut escaped, c);
                    }
                }
                *cell = escaped;
            }
            *w = (*w).max(cell.chars().count());
        }
    }
    let mut out = String::new();
    for row in rows {
        if !out.is_empty() {
            out.push('\n');
        }
        for (i, cell) in row.iter().enumerate() {
            out.push_str(cell);
            if let Some(w) = widths.get(i) {
                out.extend(std::iter::repeat_n(' ', w - cell.chars().count() + 2));
            }
        }
    }
    out
}

fn now() -> Result<Moment, Fail> {
    match env::var("LOOPS_TEST_PLAIN_NOW") {
        Ok(s) => {
            instant(&s).ok_or_else(|| usage("LOOPS_TEST_PLAIN_NOW needs a canonical UTC instant"))
        }
        Err(env::VarError::NotPresent) => clock::now()
            .and_then(|s| instant(&s))
            .ok_or_else(|| Fail::System("the clock is outside 1970–9999".into())),
        Err(env::VarError::NotUnicode(_)) => {
            Err(usage("LOOPS_TEST_PLAIN_NOW needs a canonical UTC instant"))
        }
    }
}

fn relative(at: &Moment, now: &Moment) -> String {
    let future = at > now;
    let (later, earlier) = if future { (at, now) } else { (now, at) };
    let seconds =
        later.seconds() - earlier.seconds() - i64::from(later.fraction() < earlier.fraction());
    let (n, unit) = match seconds {
        0 => return "now".into(),
        1..60 => (seconds, "second"),
        60..3600 => (seconds / 60, "minute"),
        3600..86400 => (seconds / 3600, "hour"),
        _ => (seconds / 86400, "day"),
    };
    let s = if n == 1 { "" } else { "s" };
    if future {
        format!("in {n} {unit}{s}")
    } else {
        format!("{n} {unit}{s} ago")
    }
}

pub(crate) fn thing(
    held: &Held,
    name: &str,
    records: usize,
    state: &LiveState<'_>,
    heads: &[Record],
) -> Result<String, Fail> {
    let now = now()?;
    let mut out = format!(
        "thing  {}\nrecords  {records}\nheads  {}",
        label(name),
        heads.len()
    );
    keyed_fields(&mut out, held, state, "");
    write!(out, "\nabout  {}", mentions(&thing_about(heads))).expect("write string");
    for head in heads {
        write!(
            out,
            "\nhead  {}\n  emitted  {}\n  occurred  {}",
            id_text(&head.id),
            relative(head.emitted.moment(), &now),
            head.occurred.as_ref().map_or("—", |c| c.text())
        )
        .expect("write string");
    }
    Ok(out)
}

/// Field state is read identically in the thing and keyed fold views.
fn keyed_fields(out: &mut String, held: &Held, state: &LiveState<'_>, indent: &str) {
    let fields = Fields::new(held, state);
    for k in &fields.names {
        write!(out, "\n{indent}  {}  {}", label(k), field(&fields.read(k))).expect("write string");
    }
    if state.whole().iter().any(|r| r.shown != Shown::Here) {
        write!(out, "\n{indent}other fields  unknown").expect("write string");
    }
}

fn mentions(names: &[String]) -> String {
    if names.is_empty() {
        "—".into()
    } else {
        names.iter().map(|s| text(s)).collect::<Vec<_>>().join(" ")
    }
}
