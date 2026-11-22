//! Does bare text spell a JSON number, array or object? This is a syntax
//! question: even 1e400 needs quotes. The store decoder instead constructs
//! values, requires finite numbers, and recursively decodes containers.

/// Consume the digits required by an integer part, fraction or exponent.
fn digits(s: &mut &[u8]) -> bool {
    let count = s.iter().take_while(|b| b.is_ascii_digit()).count();
    *s = &s[count..];
    count != 0
}

fn number(s: &mut &[u8]) -> bool {
    take(s, b"-");
    if !take(s, b"0") && !digits(s) {
        return false;
    }
    if take(s, b".") && !digits(s) {
        return false;
    }
    if take(s, b"e") || take(s, b"E") {
        if !take(s, b"+") {
            take(s, b"-");
        }
        return digits(s);
    }
    true
}

fn take(s: &mut &[u8], prefix: &[u8]) -> bool {
    if let Some(rest) = s.strip_prefix(prefix) {
        *s = rest;
        true
    } else {
        false
    }
}

fn whitespace(s: &mut &[u8]) {
    while matches!(s.first(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        *s = &s[1..];
    }
}

fn string(s: &mut &[u8]) -> bool {
    if !take(s, b"\"") {
        return false;
    }
    while let Some((&b, rest)) = s.split_first() {
        *s = rest;
        match b {
            b'"' => return true,
            0..=0x1f => return false,
            b'\\' => match s.split_first() {
                Some((b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't', rest)) => *s = rest,
                Some((b'u', rest))
                    if rest
                        .get(..4)
                        .is_some_and(|hex| hex.iter().all(u8::is_ascii_hexdigit)) =>
                {
                    *s = &rest[4..];
                }
                _ => return false,
            },
            _ => {}
        }
    }
    false
}

fn member(s: &mut &[u8]) -> bool {
    whitespace(s);
    if !string(s) {
        return false;
    }
    whitespace(s);
    take(s, b":")
}

pub(super) fn number_or_container(text: &str) -> bool {
    let mut s = text.as_bytes();
    if !matches!(s.first(), Some(b'[' | b'{')) {
        return matches!(s.first(), Some(b'-' | b'0'..=b'9')) && number(&mut s) && s.is_empty();
    }

    // Each open container needs only its closing delimiter, even when the
    // string being displayed contains deeply nested JSON.
    let mut ends = Vec::new();
    loop {
        whitespace(&mut s);
        match s.first().copied() {
            Some(b'[' | b'{') => {
                let end = if s[0] == b'[' { b']' } else { b'}' };
                s = &s[1..];
                whitespace(&mut s);
                if !take(&mut s, &[end]) {
                    if end == b'}' && !member(&mut s) {
                        return false;
                    }
                    ends.push(end);
                    continue;
                }
            }
            Some(b'"') if string(&mut s) => {}
            Some(b'-' | b'0'..=b'9') if number(&mut s) => {}
            Some(b't') if take(&mut s, b"true") => {}
            Some(b'f') if take(&mut s, b"false") => {}
            Some(b'n') if take(&mut s, b"null") => {}
            _ => return false,
        }
        loop {
            whitespace(&mut s);
            let Some(&end) = ends.last() else {
                return s.is_empty();
            };
            if take(&mut s, &[end]) {
                ends.pop();
            } else if take(&mut s, b",") && (end == b']' || member(&mut s)) {
                break;
            } else {
                return false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::number_or_container;

    #[test]
    fn looks_at_the_entire_json_spelling_without_decoding_the_value() {
        for text in [
            "0",
            "-0",
            "0.1",
            "1E-7",
            "[false, null, -1e900]",
            r#"{"":["\"","\\","\u00ff",{}]}"#,
            "[\r\n\t ]",
        ] {
            assert!(number_or_container(text), "{text:?}");
        }
        for text in [
            "-",
            "01",
            ".1",
            "1.",
            "+3",
            "0e-",
            "0ee2",
            "[]!",
            "[trueish]",
            "[1,]",
            "[}",
            "{1:2}",
            r#"{"k" 3}"#,
            r#"{"k": }"#,
            r#"["\q"]"#,
            r#"["\u0yy0"]"#,
            "[\"a\nb\"]",
        ] {
            assert!(!number_or_container(text), "{text:?}");
        }
        let nested = format!("{}0{}", "[".repeat(10_000), "]".repeat(10_000));
        assert!(number_or_container(&nested));
    }
}
