//! The observer's now, in the one spelling `emitted` takes.
use std::time::{SystemTime, UNIX_EPOCH};

/// Writes a moment as a canonical UTC instant: `Z`, and a fraction only when
/// it is not zero, without trailing zeros.
///
/// Returns `None` past year 9999, which `emitted` cannot spell.
pub fn instant(seconds: u64, nanos: u32) -> Option<String> {
    let seconds = i64::try_from(seconds).ok()?;
    let days = seconds.div_euclid(86_400);
    let of_day = seconds.rem_euclid(86_400);
    // Civil date from a day count (Howard Hinnant's algorithm), valid for all i64 days used here.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    if !(0..=9999).contains(&year) || nanos >= 1_000_000_000 {
        return None;
    }
    let mut text = format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        of_day / 3_600,
        of_day % 3_600 / 60,
        of_day % 60
    );
    if nanos != 0 {
        let fraction = format!("{nanos:09}");
        text.push('.');
        text.push_str(fraction.trim_end_matches('0'));
    }
    text.push('Z');
    Some(text)
}

/// Reads the system clock as a canonical instant, to the millisecond.
///
/// Returns `None` when the clock is before 1970 or past year 9999.
pub fn now() -> Option<String> {
    let since = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    instant(since.as_secs(), since.subsec_millis() * 1_000_000)
}
