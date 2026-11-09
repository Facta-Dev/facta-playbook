//! Password hashing (go `internal/platform/authx/password.go` parity).
//!
//! Deviation recorded against go: go hashes with **bcrypt cost 10** over a
//! peppered HMAC-SHA256+base64 pre-hash; this port uses **argon2id** (the
//! workspace-pinned argon2 crate, OWASP-recommended) with the SAME pepper
//! pre-hash, so the pepper semantics — pre-hash keyed by the pepper,
//! base64-encoded to defeat the (bcrypt-era) 72-byte/NUL limits and keep the
//! pepper secret from a stolen table — match go exactly. Wire semantics
//! (pepper applied, constant-time verify, identical failure envelopes) hold;
//! stored hashes are NOT byte-compatible with go's (different algorithm), so
//! a user database cannot be migrated by hash-compat alone.

use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use base64::Engine;
use <app>_core::{Error, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

/// argon2id, OWASP-recommended default parameters (m=19 MiB, t=2, p=1). The
/// stored PHC string carries the algorithm/params, so verify adapts to what
/// `hash_password` wrote without a parameter constant to keep in sync.
const ARGON2_PARAMS: Params = Params::DEFAULT;

/// HMAC-SHA256 keyed by the pepper over the password, base64-encoded — the
/// exact `preHash` of go `password.go`. An empty pepper feeds the raw
/// password bytes instead (go parity).
type HmacSha256 = Hmac<Sha256>;

fn pre_hash(password: &str, pepper: &str) -> Vec<u8> {
    if pepper.is_empty() {
        return password.as_bytes().to_vec();
    }
    let mut mac =
        HmacSha256::new_from_slice(pepper.as_bytes()).expect("hmac accepts any key length");
    mac.update(password.as_bytes());
    base64::engine::general_purpose::STANDARD
        .encode(mac.finalize().into_bytes())
        .into_bytes()
}

/// Build the stored argon2id PHC hash for `password` with `pepper` applied.
///
/// # Errors
/// [`Error::Internal`] if argon2 rejects the salt or the PHC encode fails
/// (arguably unreachable: fresh random 16-byte salt, default params).
pub fn hash_password(password: &str, pepper: &str) -> Result<String> {
    let mut salt_bytes = [0u8; 16];
    getrandom::fill(&mut salt_bytes).map_err(|e| Error::Internal(format!("password hash: {e}")))?;
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|e| Error::Internal(format!("password hash: {e}")))?;
    let preconditioned = pre_hash(password, pepper);
    let argon2 = argon2_with_pepper(pepper)?;
    argon2
        .hash_password(&preconditioned, &salt)
        .map(|h| h.to_string())
        .map_err(|e| Error::Internal(format!("password hash: {e}")))
}

/// Verify `password` against the stored PHC hash with `pepper` applied.
/// Constant-time verification (argon2's own comparison); a malformed hash or
/// a mismatch returns `false` for the caller to map to its credential error.
#[must_use]
pub fn verify_password(hash: &str, password: &str, pepper: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    let Ok(argon2) = argon2_with_pepper(pepper) else {
        return false;
    };
    argon2
        .verify_password(&pre_hash(password, pepper), &parsed)
        .is_ok()
}

/// Constant-time equality for token/code comparisons (go
/// `subtle.ConstantTimeCompare` usage). Length differences return `false`
/// immediately (go parity — length is not secret).
#[must_use]
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

fn argon2_with_pepper(pepper: &str) -> Result<Argon2<'_>> {
    if pepper.is_empty() {
        return Ok(Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            ARGON2_PARAMS,
        ));
    }
    Argon2::new_with_secret(
        pepper.as_bytes(),
        Algorithm::Argon2id,
        Version::V0x13,
        ARGON2_PARAMS,
    )
    .map_err(|e| Error::Internal(format!("password pepper: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_round_trip_with_and_without_pepper() {
        for pepper in ["a-pepper-32-bytes-or-more-x", ""] {
            let hash = hash_password("s3cret-pw", pepper).expect("hash");
            assert!(hash.starts_with("$argon2id$"), "{hash}");
            assert!(verify_password(&hash, "s3cret-pw", pepper), "right pw");
            assert!(!verify_password(&hash, "wrong", pepper), "wrong pw");
        }
    }

    #[test]
    fn password_pepper_matters() {
        let ha = hash_password("pw", "pepper-aaaaaaaaaaaaaaaaaaaaaaaaaaaa").expect("hash");
        let hb = hash_password("pw", "pepper-bbbbbbbbbbbbbbbbbbbbbbbbbbbb").expect("hash");
        assert_ne!(ha, hb, "same password, different peppers must differ");
        assert!(
            !verify_password(&ha, "pw", "pepper-bbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            "hash from pepper A must not verify under pepper B"
        );
        // Salt independence: two hashes of the same (password, pepper) both
        // verify — the argon2 salt baked into each must not affect matching.
        let ha2 = hash_password("pw", "pepper-aaaaaaaaaaaaaaaaaaaaaaaaaaaa").expect("hash");
        assert!(verify_password(
            &ha2,
            "pw",
            "pepper-aaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ));
    }

    #[test]
    fn verify_password_rejects_malformed_hash() {
        assert!(!verify_password("not-a-phc-string", "pw", ""));
        assert!(!verify_password("", "pw", ""));
    }

    #[test]
    fn hashes_are_not_bcrypt_compatible() {
        // The deviation marker: an argon2id hash never parses as go's bcrypt
        // string, and verify_password refuses it rather than mis-verifying.
        let bcrypt_like = "$2a$10$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy";
        assert!(!verify_password(bcrypt_like, "pw", ""));
    }

    #[test]
    fn constant_time_eq_matches_go_semantics() {
        assert!(constant_time_eq("123456", "123456"));
        assert!(!constant_time_eq("123456", "123457"));
        assert!(!constant_time_eq("123456", "12345")); // length differs
        assert!(constant_time_eq("", ""));
    }
}
