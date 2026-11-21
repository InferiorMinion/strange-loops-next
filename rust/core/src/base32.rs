//! RFC 4648 encoding from model/Loops/Base32.lean.
/// Lean: `Loops.base32`, model/Loops/Base32.lean.
/// Encodes bytes with lowercase base32hex and no padding characters.
/// Adds zero bits on the right for the final group.
pub fn base32(bytes: &[u8]) -> String {
    digits(bytes).map(char::from).collect()
}

pub(crate) fn matches(bytes: &[u8], text: &str) -> bool {
    text.len() == (bytes.len() * 8).div_ceil(5) && digits(bytes).eq(text.bytes())
}

fn digits(bytes: &[u8]) -> impl Iterator<Item = u8> + '_ {
    const ALPHABET: &[u8; 32] = b"0123456789abcdefghijklmnopqrstuv";
    let mut input = bytes.iter();
    let mut acc = 0_u16;
    let mut bits = 0;
    std::iter::from_fn(move || {
        if bits < 5 {
            if let Some(&byte) = input.next() {
                acc = (acc << 8) | u16::from(byte);
                bits += 8;
            } else if bits > 0 {
                acc <<= 5 - bits;
                bits = 5;
            } else {
                return None;
            }
        }
        bits -= 5;
        Some(ALPHABET[usize::from((acc >> bits) & 31)])
    })
}

/// Decodes only canonical lowercase base32hex text.
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let mut acc = 0_u16;
    let mut bits = 0;
    let mut bytes = Vec::new();
    for c in text.bytes() {
        let digit = b"0123456789abcdefghijklmnopqrstuv"
            .iter()
            .position(|b| *b == c)?;
        acc = (acc << 5) | u16::try_from(digit).ok()?;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            bytes.push(u8::try_from((acc >> bits) & 255).ok()?);
        }
        acc &= (1 << bits) - 1;
    }
    (base32(&bytes) == text).then_some(bytes)
}

#[cfg(test)]
mod tests {
    use super::{base32, decode};

    #[test]
    fn fixed_width_padding_and_order() {
        for (n, bits) in [(16, 2), (32, 4), (64, 3)] {
            let zero = vec![0; n];
            let encoded = base32(&zero);
            assert_eq!(decode(&encoded), Some(zero));
            for digit in 1..(1 << bits) {
                let mut invalid = encoded.clone();
                invalid.pop();
                invalid.push(char::from(b"0123456789abcdefghijklmnopqrstuv"[digit]));
                assert_eq!(decode(&invalid), None, "{invalid}");
            }
            assert!(decode(&encoded.to_uppercase().replace('0', "W")).is_none());
            assert!(decode(&format!("{encoded}=")).is_none());
        }
        let mut values = vec![vec![0; 32], vec![255; 32]];
        for i in 0..32 {
            let mut x = vec![0; 32];
            x[i] = 128;
            values.push(x);
        }
        values.sort();
        let texts: Vec<_> = values.iter().map(|v| base32(v)).collect();
        assert!(texts.windows(2).all(|w| w[0] < w[1]));
    }
}
