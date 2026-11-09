//! A device's signing key, and the randomness a writer needs.
use ed25519_dalek::{Signer, SigningKey};
use loops::base32;
use std::fmt;

/// An Ed25519 signing key, held as its 32-byte seed.
#[derive(Clone)]
pub struct Key(SigningKey);

impl fmt::Debug for Key {
    /// Shows the public key only; the seed never reaches a log.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Key").field(&self.public()).finish()
    }
}

impl Key {
    /// Makes a new key from the system's randomness.
    ///
    /// # Errors
    /// Returns the system's error when it has no randomness to give.
    pub fn generate() -> Result<Self, getrandom::Error> {
        let mut seed = [0_u8; 32];
        getrandom::getrandom(&mut seed)?;
        Ok(Self(SigningKey::from_bytes(&seed)))
    }
    /// Reads a seed written by [`Key::text`]: 52 canonical base32hex characters.
    /// Surrounding whitespace is ignored, so a key file may end in a newline.
    pub fn from_text(text: &str) -> Option<Self> {
        let seed: [u8; 32] = base32::decode(text.trim()).and_then(|b| b.try_into().ok())?;
        Some(Self(SigningKey::from_bytes(&seed)))
    }
    /// Writes the seed. Whoever holds this text signs as this key.
    pub fn text(&self) -> String {
        base32::base32(&self.0.to_bytes())
    }
    /// The public key as a stored line names it: `k:` and base32hex.
    pub fn public(&self) -> String {
        format!("k:{}", base32::base32(&self.0.verifying_key().to_bytes()))
    }
    pub(crate) fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.0.sign(message).to_bytes()
    }
}

/// Makes a record's salt: 16 random bytes.
///
/// # Errors
/// Returns the system's error when it has no randomness to give.
pub fn salt() -> Result<Vec<u8>, getrandom::Error> {
    let mut salt = [0_u8; 16];
    getrandom::getrandom(&mut salt)?;
    Ok(salt.to_vec())
}
