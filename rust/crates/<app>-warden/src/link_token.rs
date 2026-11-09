//! The `?state=` JWT of the GitHub account-linking flow (go
//! `internal/platform/oauthx/linktoken.go` parity): `{sub, purpose}`,
//! HS256 over the shared API secret, minted by the connect-url endpoint and
//! verified back on the callback. The `purpose` claim keeps access tokens
//! from double-clicking as link states.

use <app>_core::{Error, Result};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

/// Link-state TTL — reference `LINK_STATE_EXP_SEC` (600s).
pub const LINK_STATE_TTL_SECS: i64 = 600;

/// Purpose claim guarding the flow (go `github_link`).
const GITHUB_LINK_PURPOSE: &str = "github_link";

#[derive(Serialize, Deserialize)]
struct LinkClaims {
    sub: String,
    purpose: String,
    exp: i64,
}

/// Mints and parses the link-state JWT over the shared secret
/// (go `oauthx.LinkToken`).
#[derive(Clone)]
pub struct LinkToken {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl std::fmt::Debug for LinkToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkToken").finish_non_exhaustive()
    }
}

impl LinkToken {
    /// Build the codec over the API's HS256 secret
    /// (`<APP>_JWT_SECRET`).
    #[must_use]
    pub fn new(secret: &str) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    /// Sign `{sub, purpose, exp = now + LINK_STATE_TTL_SECS}`.
    ///
    /// # Errors
    /// [`Error::Internal`] if encoding fails (effectively never).
    pub fn mint_link(&self, user_id: &str) -> Result<String> {
        let exp = crate::current_unix_secs() + LINK_STATE_TTL_SECS;
        let claims = LinkClaims {
            sub: user_id.to_owned(),
            purpose: GITHUB_LINK_PURPOSE.into(),
            exp,
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| Error::Internal(format!("link state encode: {e}")))
    }

    /// Return the subject of a valid link-state token; anything else (wrong
    /// signature, expired, wrong/missing purpose, empty subject) is
    /// [`Error::Unauthenticated`] ("invalid link state").
    ///
    /// # Errors
    /// As above — every refusal maps there.
    pub fn parse_link(&self, state: &str) -> Result<String> {
        let validation = Validation::new(Algorithm::HS256);
        let data =
            decode::<LinkClaims>(state, &self.decoding, &validation).map_err(|_| invalid())?;
        if data.claims.purpose != GITHUB_LINK_PURPOSE || data.claims.sub.is_empty() {
            return Err(invalid());
        }
        Ok(data.claims.sub)
    }
}

fn invalid() -> Error {
    Error::Unauthenticated("invalid link state".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_parse_roundtrip() {
        let token = LinkToken::new("0123456789abcdef0123456789abcdef");
        let state = token.mint_link("u-1").expect("mint");
        assert_eq!(token.parse_link(&state).expect("parse"), "u-1");
    }

    #[test]
    fn token_without_purpose_is_refused() {
        use jsonwebtoken::encode;
        // A same-secret access-token-shaped JWT (no purpose claim) must not
        // pass as a link state.
        #[derive(Serialize)]
        struct NoPurpose {
            sub: String,
            exp: i64,
        }
        let secret = "0123456789abcdef0123456789abcdef";
        let forged = encode(
            &Header::new(Algorithm::HS256),
            &NoPurpose {
                sub: "u-1".into(),
                exp: crate::current_unix_secs() + 3600,
            },
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("forge");
        let token = LinkToken::new(secret);
        assert!(token.parse_link(&forged).is_err(), "purpose is required");
    }

    #[test]
    fn expired_and_garbage_states_are_refused() {
        let token = LinkToken::new("0123456789abcdef0123456789abcdef");
        // Mint with the TTL negated so exp is already past.
        let expired_claims = LinkClaims {
            sub: "u-1".into(),
            purpose: GITHUB_LINK_PURPOSE.into(),
            exp: crate::current_unix_secs() - LINK_STATE_TTL_SECS,
        };
        let expired = encode(
            &Header::new(Algorithm::HS256),
            &expired_claims,
            &EncodingKey::from_secret(b"0123456789abcdef0123456789abcdef"),
        )
        .expect("mint expired");
        let err = token.parse_link(&expired).unwrap_err();
        assert!(matches!(err, Error::Unauthenticated(_)), "{err}");
        assert!(token.parse_link("garbage").is_err());
    }

    #[test]
    fn wrong_secret_is_refused() {
        let mint_side = LinkToken::new("secret-one-0123456789abcdef0123456789abcd");
        let verify_side = LinkToken::new("secret-two-0123456789abcdef0123456789abcd");
        let state = mint_side.mint_link("u-1").expect("mint");
        assert!(verify_side.parse_link(&state).is_err());
    }
}
