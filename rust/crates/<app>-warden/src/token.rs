//! Token material (go `internal/shared/idgen` parity): opaque unguessable
//! tokens for session refresh and numeric verification codes.
//!
//! Offline-guarded: crypto-grade OS randomness only (getrandom), no socket,
//! no clock. The 6-digit codes ride the same generator go uses
//! (`crypto/rand.Int` — uniform, no modulo bias).

use base64::Engine;
use <app>_core::{Error, Result};

/// A 64-char URL-safe opaque token (48 random bytes) — go `idgen.NewToken`
/// parity. Used for session refresh tokens and, in the social flows, for the
/// provisioned user's unusable random password.
///
/// # Errors
/// [`Error::Internal`] if the OS entropy source fails (effectively never).
pub fn new_token() -> Result<String> {
    let mut bytes = [0u8; 48];
    getrandom::fill(&mut bytes).map_err(|e| Error::Internal(format!("token generation: {e}")))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

/// An n-digit numeric code, uniformly sampled (go `idgen.NewCode` parity).
/// The signup/password flows use 6-digit codes. Bytes >= 250 are rejected
/// (rejection sampling) so every digit is uniform over 0-9.
///
/// # Errors
/// [`Error::Internal`] if the OS entropy source fails (effectively never).
pub fn new_code(n: usize) -> Result<String> {
    let mut digits = Vec::with_capacity(n);
    let mut byte = [0u8; 1];
    while digits.len() < n {
        getrandom::fill(&mut byte).map_err(|e| Error::Internal(format!("code generation: {e}")))?;
        if byte[0] < 250 {
            digits.push(b'0' + (byte[0] % 10));
        }
    }
    // All bytes < 250 yield a uniform digit, so the string is pure ASCII —
    // an entropy failure surfaces through the map_err above, never here.
    String::from_utf8(digits).map_err(|e| Error::Internal(format!("code ascii: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_64_url_safe_chars() {
        let token = new_token().expect("token");
        assert_eq!(token.len(), 64, "48 bytes base64url-encoded is 64 chars");
        assert!(
            token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "url-safe alphabet: {token}"
        );
        assert_ne!(new_token().expect("token"), token, "tokens are unique");
    }

    #[test]
    fn code_is_n_uniform_digits() {
        let code = new_code(6).expect("code");
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()), "{code}");
        assert_eq!(new_code(8).expect("code").len(), 8);
        // Distribution sanity: over many codes each digit position hits
        // several values (uniform sampling would take 10^6 runs to prove
        // uniformity; this only catches a stuck/biased generator).
        let mut seen = std::collections::HashSet::new();
        for _ in 0..200 {
            let c = new_code(1).expect("code");
            seen.insert(c);
        }
        assert!(seen.len() >= 9, "digit coverage too narrow: {seen:?}");
    }

    #[test]
    fn code_with_zero_digits_is_empty() {
        assert_eq!(new_code(0).expect("code"), "");
    }
}
