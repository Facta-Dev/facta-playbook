//! JWT bearer authentication. Verifies an HS256 access token, resolves a
//! [`Principal`] with roles, and attaches it to the request extensions so
//! handlers can authorize via `<app>-core::rbac::can`.
//!
//! Identity-core parity (go `internal/platform/authx` + `oauthx`): the claims
//! widen to the go set (`email`/`role`/`user_type` alongside the rust
//! `roles` snapshot), password hashing lives in [`password`] (argon2id over
//! the go pepper pre-hash — recorded deviation: go uses bcrypt), token
//! material in [`token`] (opaque refresh tokens, 6-digit codes, constant-time
//! equality), and the OAuth clients in [`google`]/[`github`]/[`link_token`].
//! Everything here is offline-guarded: no socket at build or test time.
//!
//! This crate is the JWT warden: a self-contained HS256 codec + claims + RBAC
//! principal + axum auth middleware. It depends only on `<app>-core` (for
//! `Role`/`Error`/`Result`) and axum — never on the API crate, so the API can
//! depend on the warden without a circular import.

pub mod cookie;
pub mod error;
pub mod github;
pub mod google;
pub mod link_token;
pub mod password;
pub mod token;

pub use cookie::{clear_auth_cookie, set_auth_cookie};
pub use error::{AuthError, ErrorBody, ErrorEnvelope};
pub use password::{constant_time_eq, hash_password, verify_password};
pub use token::{new_code, new_token};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::{
    extract::Request,
    http::{HeaderMap, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Validation, decode, decode_header};
// `Header` and `encode` are only used by `mint`, which is gated behind
// `#[cfg(any(test, feature = "mint"))]`. Gate the imports to match so a
// no-mint build has no unused-import warnings.
use <app>_core::{Error, Result, Role};
#[cfg(any(test, feature = "mint"))]
use jsonwebtoken::{Header, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Audience every access token is minted with and validated against. A token
/// minted for another service (different `aud`) is rejected.
pub const ACCESS_AUDIENCE: &str = "<app>-api";

/// Issuer stamped on every token and checked on verify.
pub const ACCESS_ISSUER: &str = "<app>";

/// Hard ceiling on access-token lifetime (`exp - iat`, seconds). `mint`
/// rejects anything longer so a leaked token stays bounded; revocation
/// ([`SessionStore`]) closes the gap within this window. 900s = 15min.
pub const MAX_ACCESS_TTL_SECS: i64 = 900;

/// Leeway (seconds) applied to revocation-entry pruning in
/// [`InMemorySessionStore`]. A revoked `jti` is evicted once `exp + leeway`
/// is in the past — the token can no longer pass `verify` at that point, so
/// keeping its revocation record is pure waste. 60s absorbs minor clock skew
/// between issuer and verifier without prematurely dropping a record.
pub const REVOCATION_LEEWAY_SECS: i64 = 60;

// L10: clock-skew leeway on `verify` is deliberately 0 (strict) — the
// `jsonwebtoken` `Validation` struct uses its own `leeway` (default 0s) for
// `exp`/`nbf` checks. An instantiator can add skew tolerance by constructing a
// custom `Validation` with `set_leeway(...)` if needed. Shipped strict to fail
// closed; do NOT loosen without a reason.
//
// L11: secrets are NOT zeroized on drop. The signing/verifying keys
// (`EncodingKey`/`DecodingKey`) live in heap memory freed by the allocator
// without overwrite — a known Rust limitation. Adding `secrecy`/`zeroize` is
// out of scope (ponytail: avoid new deps); instantiators with strict
// memory-hygiene requirements should wrap the codec in a process boundary.

/// JWT claims. `roles` is embedded so the auth middleware can build a
/// [`Principal`] and run `can()` without a DB read — a snapshot at most one
/// access-TTL stale. `jti` is a unique per-token id used for revocation —
/// go parity: `jti` is the session id, so revoking the session revokes the
/// token. `email`/`role`/`user_type` are the go claim set (go
/// `internal/platform/authx`): `role`/`user_type` carry the DB strings
/// (`ADMIN`/`MODERATOR`/`USER`/`GUEST`, `ADMIN`/`COMPANY`/`USER`), stamped by
/// [`JwtCodec::mint_identity`] and absent on tokens minted by the plain
/// [`JwtCodec::mint`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the caller's id.
    pub sub: Uuid,
    /// Roles granted to the caller. Drives RBAC.
    pub roles: Vec<Role>,
    /// Email claim (go `email`), when minted on the login path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Account role claim as the go DB string (go `role`), when minted on
    /// the login path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Account type claim as the go DB string (go `user_type` per the python
    /// reference), when minted on the login path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,
    /// Audience — the service this token is intended for.
    pub aud: String,
    /// Issuer.
    pub iss: String,
    /// Issued-at (unix seconds).
    pub iat: i64,
    /// Expiry (unix seconds).
    pub exp: i64,
    /// JWT id — unique per token, used for revocation lookups via
    /// [`SessionStore`]. Required; tokens without `jti` are rejected on verify.
    /// Login-path tokens stamp the session id here (go parity).
    pub jti: String,
}

/// Identity stamped into a minted access token — the login-path input of
/// [`JwtCodec::mint_identity`] (go `authx.Identity` field-for-field, with the
/// rust `roles` RBAC snapshot added beside the go `role` DB string).
#[derive(Debug, Clone)]
pub struct Identity {
    pub user_id: Uuid,
    pub email: Option<String>,
    /// go DB role string (`ADMIN`|`MODERATOR`|`USER`|`GUEST`).
    pub role: Option<String>,
    /// go DB user-type string (`ADMIN`|`COMPANY`|`USER`).
    pub user_type: Option<String>,
    /// JWT id — the session id (go parity: `jti == sessions.id`, so
    /// deactivating the session is the revocation of the access token).
    pub jti: String,
    /// Rust RBAC snapshot for the `roles` claim (see [`Claims::roles`]).
    pub roles: Vec<Role>,
}

/// Authenticated caller, derived from a verified access token. No DB hit.
/// Set by [`require_auth`] into request extensions; extracted by handlers via
/// `Extension<Principal>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    /// Caller's id (JWT `sub`).
    pub id: Uuid,
    /// Roles granted by the token.
    pub roles: Vec<Role>,
}

/// Generic 401 message used by [`JwtCodec::verify`]. The specific
/// `jsonwebtoken`/`kid`/revocation reason is logged at `tracing::debug!` and
/// never echoed to the client body (the `Error::Unauthenticated` string is
/// surfaced verbatim by `ApiError` for 4xx, so it must stay generic).
fn unauthenticated_generic() -> Error {
    Error::Unauthenticated("invalid access token".into())
}

/// Current unix time in seconds. Shared by [`InMemorySessionStore`] pruning
/// and [`JwtCodec::verify`] iat/future checks. Falls back to 0 if the system
/// clock is before the epoch (should never happen on a real host).
#[allow(clippy::cast_possible_wrap)]
fn current_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Pluggable session/jti revocation oracle. [`JwtCodec::verify`] consults the
/// store and rejects any token whose `jti` is revoked, closing the
/// no-revocation gap left by a pure-signature verify.
///
/// `Send + Sync` so it can sit behind an `Arc<dyn SessionStore>` in shared
/// `AppState`. Wire a persistent/distributed store (Redis, DB, …) in
/// production; [`InMemorySessionStore`] is the default for tests and
/// single-instance deployments.
///
/// M1: `revoke` is part of the trait contract so an instantiator's persistent
/// store implements the SAME revocation surface as the in-memory default —
/// callers (`require_auth`, a future logout handler) never need to downcast.
/// The `exp` parameter lets the store bound its own storage (the in-memory
/// store prunes entries whose token has expired; a Redis store can set a TTL).
pub trait SessionStore: Send + Sync {
    /// True if the token id (`jti`) has been revoked and must no longer be
    /// honored.
    fn is_revoked(&self, jti: &str) -> bool;

    /// Mark a `jti` as revoked. `exp` is the token's expiry (unix seconds) so
    /// the store can evict the record once the token is no longer valid.
    ///
    /// # Errors
    /// A persistent store may return [`Error::Internal`] if the backing
    /// write fails (DB/Redis down). The in-memory default is infallible.
    fn revoke(&self, jti: &str, exp: i64) -> Result<()>;
}

/// Default in-process revocation store. Tracks `(jti, exp)` pairs so expired
/// revocation records are pruned (bounded memory). Not durable across
/// restarts — swap in a persistent store via [`JwtCodec::with_store`] for
/// production. M1: pruning runs on every `revoke` AND `is_revoked` call, so
/// the set stays bounded under sustained revocation traffic.
#[derive(Debug, Default)]
pub struct InMemorySessionStore {
    revoked: Mutex<HashMap<String, i64>>,
}

impl InMemorySessionStore {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Evict entries whose token has expired (exp + leeway < now). Called on
    /// every `revoke` and `is_revoked` so the set cannot grow unbounded.
    fn prune_expired(entries: &mut HashMap<String, i64>, now: i64) {
        entries.retain(|_, exp| *exp + REVOCATION_LEEWAY_SECS >= now);
    }
}

impl SessionStore for InMemorySessionStore {
    fn is_revoked(&self, jti: &str) -> bool {
        let mut entries = self.revoked.lock().expect("revoked set lock poisoned");
        Self::prune_expired(&mut entries, current_unix_secs());
        entries.contains_key(jti)
    }

    fn revoke(&self, jti: &str, exp: i64) -> Result<()> {
        {
            let mut entries = self.revoked.lock().expect("revoked set lock poisoned");
            Self::prune_expired(&mut entries, current_unix_secs());
            entries.insert(jti.to_string(), exp);
        }
        Ok(())
    }
}

/// HS256 JWT codec — encodes (mint) and decodes (verify) access tokens.
/// Constructed once at boot from `<APP>_JWT_SECRET` and shared via `AppState`.
///
/// Key rotation: [`JwtCodec::new`] accepts a set of `(kid, secret)` pairs. The
/// first pair's `kid` is stamped into the header on `mint`; every pair is
/// accepted on `verify`, so an old key keeps working until the rotated key is
/// dropped from the set. Tokens with a missing or unknown `kid` are rejected —
/// a valid signature alone is not enough.
#[derive(Clone)]
pub struct JwtCodec {
    /// `kid` stamped into the header on `mint`; rotated by reconstructing the
    /// codec with the new key first. Only read by `mint` (gated behind
    /// `mint` feature); `#[allow(dead_code)]` suppresses the warning in a
    /// production build where `mint` is off.
    #[cfg_attr(not(any(test, feature = "mint")), allow(dead_code))]
    current_kid: String,
    /// Encoding key used by `mint`. Same gating as `current_kid`.
    #[cfg_attr(not(any(test, feature = "mint")), allow(dead_code))]
    current_encoding: EncodingKey,
    /// Every accepted `(kid, DecodingKey)` pair for verify.
    decoding_keys: HashMap<String, DecodingKey>,
    /// Revocation oracle consulted on every verify. Defaults to
    /// [`InMemorySessionStore`]; swap via [`JwtCodec::with_store`].
    store: Arc<dyn SessionStore>,
}

impl std::fmt::Debug for JwtCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the signing keys.
        f.debug_struct("JwtCodec").finish_non_exhaustive()
    }
}

impl JwtCodec {
    /// Construct from a set of `(kid, secret)` pairs for key rotation. The
    /// first pair's `kid` is stamped on newly minted tokens; every pair is
    /// accepted on verify (so old keys keep working during rotation). HS256
    /// only — [`Validation`] pins `HS256` and `verify` rejects any other alg,
    /// so no algorithm confusion is possible.
    ///
    /// L1: duplicate `kid` values are detected and rejected — a duplicate
    /// would silently shadow one of the keys during `into_iter().collect()`,
    /// making rotation unpredictable.
    ///
    /// M4: multi-key rotation requires the instantiator to wire their own
    /// config format (a `kid`-keyed map of secrets) and call this constructor
    /// with the full set. The shipped [`Config`](crate::Config) / `from_env`
    /// path uses [`JwtCodec::from_single`] (a single `kid = "default"` key) —
    /// it does NOT wire rotation out of the box. Do not assume rotation works
    /// without custom wiring.
    ///
    /// # Panics
    /// Panics if `keys` is empty (a codec with no keys is unusable), or if two
    /// pairs share the same `kid` (a duplicate would silently shadow a key).
    #[must_use]
    pub fn new(keys: Vec<(String, Vec<u8>)>) -> Self {
        assert!(
            !keys.is_empty(),
            "JwtCodec::new requires at least one (kid, secret) pair"
        );
        let mut seen = std::collections::HashSet::new();
        for (kid, _) in &keys {
            assert!(
                seen.insert(kid.as_str()),
                "JwtCodec::new: duplicate kid '{kid}' — each key must have a unique kid"
            );
        }
        let current_kid = keys[0].0.clone();
        let current_encoding = EncodingKey::from_secret(&keys[0].1);
        let decoding_keys = keys
            .into_iter()
            .map(|(kid, secret)| (kid, DecodingKey::from_secret(&secret)))
            .collect();
        Self {
            current_kid,
            current_encoding,
            decoding_keys,
            store: Arc::new(InMemorySessionStore::new()),
        }
    }

    /// Convenience for the single-key case: one pair with `kid = "default"`.
    /// Used by `Config` wiring and tests where rotation is not exercised.
    /// M4: the shipped `from_env` config path calls this — no rotation. See
    /// [`JwtCodec::new`] for the rotation wiring contract.
    #[must_use]
    pub fn from_single(secret: &[u8]) -> Self {
        Self::new(vec![("default".to_string(), secret.to_vec())])
    }

    /// Swap the default [`InMemorySessionStore`] for a persistent/distributed
    /// revocation store (Redis, DB, …). Builder-style.
    #[must_use]
    pub fn with_store(mut self, store: Arc<dyn SessionStore>) -> Self {
        self.store = store;
        self
    }

    /// Mint a signed access token for `sub` with `roles`. Used by tests and a
    /// future login handler. Stamps a `kid` (current key id) and a random
    /// `jti` (for revocation) into the token.
    ///
    /// Gated behind `#[cfg(any(test, feature = "mint"))]`. The `mint`
    /// feature is OFF by default on the warden crate; the api crate enables it
    /// only under `[dev-dependencies]` (test builds). A production build has
    /// NO `mint` callable — `JwtCodec` in `AppState` is verify-only. This
    /// makes the capability separation physical: a request handler in a
    /// production build cannot mint a token even if it wanted to. A future
    /// login handler must be built behind `#[cfg(feature = "mint")]` (or in a
    /// separate binary that enables the feature).
    ///
    /// M2: TTL arithmetic uses `checked_sub` so an out-of-range `iat`/`exp`
    /// pair is rejected with a typed [`Error::Invalid`] instead of panicking
    /// on overflow in debug / wrapping silently in release.
    ///
    /// # Errors
    /// - [`Error::Invalid`] if `exp < iat` or if `exp - iat` overflows.
    /// - [`Error::Invalid`] if `exp - iat` exceeds [`MAX_ACCESS_TTL_SECS`].
    /// - [`Error::Internal`] if `jsonwebtoken` encoding fails.
    #[cfg(any(test, feature = "mint"))]
    pub fn mint(&self, sub: Uuid, roles: Vec<Role>, iat: i64, exp: i64) -> Result<String> {
        if exp < iat {
            return Err(Error::Invalid("jwt mint: exp must be >= iat".into()));
        }
        let Some(ttl) = exp.checked_sub(iat) else {
            return Err(Error::Invalid(
                "jwt mint: iat/exp arithmetic overflow".into(),
            ));
        };
        if ttl > MAX_ACCESS_TTL_SECS {
            return Err(Error::Invalid(format!(
                "jwt mint: access TTL {ttl}s exceeds max {MAX_ACCESS_TTL_SECS}s"
            )));
        }
        let claims = Claims {
            sub,
            roles,
            email: None,
            role: None,
            user_type: None,
            aud: ACCESS_AUDIENCE.to_string(),
            iss: ACCESS_ISSUER.to_string(),
            iat,
            exp,
            jti: Uuid::new_v4().to_string(),
        };
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(self.current_kid.clone());
        encode(&header, &claims, &self.current_encoding)
            .map_err(|e| Error::Internal(format!("jwt encode: {e}")))
    }

    /// Mint an access token from a login-path [`Identity`] — go
    /// `authx.Mint` parity: the same claim set plus `email`, the go DB `role`
    /// string, `user_type`, and `jti` stamped with the **session id** so that
    /// deactivating the session is the revocation of the access token (go
    /// `login` mints with `jti: sessionID`).
    ///
    /// All mint-side TTL rules are identical to [`JwtCodec::mint`].
    ///
    /// Gated behind `#[cfg(any(test, feature = "mint"))]` exactly like
    /// [`JwtCodec::mint`] — production builds have no login-path mint.
    ///
    /// # Errors
    /// Same TTL errors as [`JwtCodec::mint`].
    #[cfg(any(test, feature = "mint"))]
    pub fn mint_identity(&self, identity: &Identity, iat: i64, exp: i64) -> Result<String> {
        if exp < iat {
            return Err(Error::Invalid("jwt mint: exp must be >= iat".into()));
        }
        let Some(ttl) = exp.checked_sub(iat) else {
            return Err(Error::Invalid(
                "jwt mint: iat/exp arithmetic overflow".into(),
            ));
        };
        if ttl > MAX_ACCESS_TTL_SECS {
            return Err(Error::Invalid(format!(
                "jwt mint: access TTL {ttl}s exceeds max {MAX_ACCESS_TTL_SECS}s"
            )));
        }
        let claims = Claims {
            sub: identity.user_id,
            roles: identity.roles.clone(),
            email: identity.email.clone(),
            role: identity.role.clone(),
            user_type: identity.user_type.clone(),
            aud: ACCESS_AUDIENCE.to_string(),
            iss: ACCESS_ISSUER.to_string(),
            iat,
            exp,
            jti: identity.jti.clone(),
        };
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(self.current_kid.clone());
        encode(&header, &claims, &self.current_encoding)
            .map_err(|e| Error::Internal(format!("jwt encode: {e}")))
    }

    /// Verify a token's signature, expiry, issuer, audience, `kid`, alg, and
    /// revocation status. Returns the claims on success.
    ///
    /// The inner `jsonwebtoken`/`kid`/revocation reason is logged at
    /// `tracing::debug!` and never echoed to the 401 body — callers receive a
    /// generic "invalid access token" message.
    ///
    /// # Errors
    /// Returns [`Error::Unauthenticated`] if the token is malformed, has a
    /// non-HS256 alg, a missing/unknown `kid`, a bad signature/expiry/issuer/
    /// audience, its `jti` is revoked, or (H1) its TTL exceeds
    /// [`MAX_ACCESS_TTL_SECS`], `iat` is in the future, or `exp < iat`.
    pub fn verify(&self, token: &str) -> Result<Claims> {
        // Peek the header first to resolve the key by `kid` and pin HS256.
        let header = decode_header(token).map_err(|e| {
            tracing::debug!(error = %e, "jwt verify: header decode failed");
            unauthenticated_generic()
        })?;
        if header.alg != Algorithm::HS256 {
            tracing::debug!(alg = ?header.alg, "jwt verify: rejected non-HS256 alg");
            return Err(unauthenticated_generic());
        }
        let kid = header.kid.as_deref().ok_or_else(|| {
            tracing::debug!("jwt verify: rejected token without kid");
            unauthenticated_generic()
        })?;
        let decoding = self.decoding_keys.get(kid).ok_or_else(|| {
            tracing::debug!(kid = kid, "jwt verify: rejected unknown kid");
            unauthenticated_generic()
        })?;
        let mut v = Validation::new(Algorithm::HS256);
        v.set_issuer(&[ACCESS_ISSUER]);
        v.set_audience(&[ACCESS_AUDIENCE]);
        let data = decode::<Claims>(token, decoding, &v).map_err(|e| {
            tracing::debug!(error = %e, "jwt verify: decode failed");
            unauthenticated_generic()
        })?;
        // L4: jti must be <= 64 chars (a UUID is 36). An oversized jti is
        // suspicious — reject before the revocation lookup to bound storage.
        if data.claims.jti.len() > 64 {
            tracing::debug!(
                jti_len = data.claims.jti.len(),
                "jwt verify: rejected oversized jti"
            );
            return Err(unauthenticated_generic());
        }
        // H1: verify-side TTL + iat/exp guards. The mint-side check (above)
        // is bypassable by constructing a token directly — verify MUST
        // re-assert the ceiling so a leaked signing key still produces
        // bounded-damage tokens. M2: checked arithmetic so overflow is
        // rejected, not wrapped.
        if data.claims.exp < data.claims.iat {
            tracing::debug!("jwt verify: rejected exp < iat");
            return Err(unauthenticated_generic());
        }
        let Some(ttl) = data.claims.exp.checked_sub(data.claims.iat) else {
            tracing::debug!("jwt verify: rejected iat/exp arithmetic overflow");
            return Err(unauthenticated_generic());
        };
        if ttl > MAX_ACCESS_TTL_SECS {
            tracing::debug!(
                ttl,
                max = MAX_ACCESS_TTL_SECS,
                "jwt verify: rejected TTL exceeds max"
            );
            return Err(unauthenticated_generic());
        }
        let now = current_unix_secs();
        // L10: strict (no leeway) on future-iat. A token from the future is
        // suspicious; the mint side stamps iat=now. Tolerance is YAGNI here.
        if data.claims.iat > now {
            tracing::debug!(
                iat = data.claims.iat,
                now,
                "jwt verify: rejected future iat"
            );
            return Err(unauthenticated_generic());
        }
        if self.store.is_revoked(&data.claims.jti) {
            tracing::debug!(jti = %data.claims.jti, "jwt verify: rejected revoked jti");
            return Err(unauthenticated_generic());
        }
        Ok(data.claims)
    }

    /// A codec with a fixed test key. All tests that need to mint/verify share
    /// this so token verification is deterministic.
    #[cfg(test)]
    #[must_use]
    pub fn test_codec() -> Self {
        Self::from_single(b"test-secret-key-at-least-32-bytes!!")
    }
}

/// Read the bearer token from `Authorization: Bearer …`.
/// L2: the scheme match is case-insensitive per RFC 7235 (`Bearer`, `bearer`,
/// `BEARER` are all valid). Splits on the first space and trims the remainder.
fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let v = headers.get(AUTHORIZATION).and_then(|h| h.to_str().ok())?;
    let (scheme, rest) = v.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = rest.trim();
    if token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

/// axum middleware: require a valid JWT bearer token. On success, attaches a
/// [`Principal`] to the request extensions. On failure, returns an
/// [`AuthError`] (401). Routes that don't need auth skip this layer.
///
/// The codec is provided via `from_fn_with_state(codec, require_auth)` —
/// independent of the router's `AppState`.
///
/// L6: the `x-request-id` header (set by the outer `request_id_layer`) is
/// pulled into the [`AuthError`] so the 401 envelope carries the correlation
/// id. The warden crate cannot import the api crate's `RequestId` type (that
/// would be a circular dep), so it reads the header directly — the
/// `request_id_layer` stamps both the extension AND the header on the request
/// before this middleware runs.
pub async fn require_auth(
    axum::extract::State(codec): axum::extract::State<JwtCodec>,
    mut req: Request,
    next: Next,
) -> std::result::Result<Response, AuthError> {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let token = extract_bearer(req.headers()).ok_or_else(|| {
        AuthError::new(Error::Unauthenticated(
            "missing access token (Authorization: Bearer …)".into(),
        ))
        .with_request_id(request_id.clone())
    })?;
    let claims = codec
        .verify(&token)
        .map_err(|e| AuthError::new(e).with_request_id(request_id.clone()))?;
    req.extensions_mut().insert(Principal {
        id: claims.sub,
        roles: claims.roles,
    });
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;

    /// Unix seconds for "now" — avoids pulling chrono into dev-deps.
    #[allow(clippy::cast_possible_wrap)]
    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_secs() as i64
    }

    async fn echo_principal(axum::Extension(p): axum::Extension<Principal>) -> String {
        p.id.to_string()
    }

    fn protected_app() -> Router {
        Router::new()
            .route("/me", get(echo_principal))
            .layer(axum::middleware::from_fn_with_state(
                JwtCodec::test_codec(),
                require_auth,
            ))
    }

    #[test]
    fn mint_and_verify_roundtrip() {
        let codec = JwtCodec::test_codec();
        let sub = Uuid::new_v4();
        let t = now();
        let token = codec
            .mint(sub, vec![Role::Owner], t, t + 900)
            .expect("mint");
        let claims = codec.verify(&token).expect("verify");
        assert_eq!(claims.sub, sub);
        assert_eq!(claims.roles, [Role::Owner]);
        assert_eq!(claims.iss, ACCESS_ISSUER);
        assert_eq!(claims.aud, ACCESS_AUDIENCE);
        assert!(!claims.jti.is_empty(), "jti must be stamped");
    }

    #[test]
    fn mint_stamps_kid() {
        let codec = JwtCodec::new(vec![(
            "current".to_string(),
            b"test-secret-key-at-least-32-bytes!!".to_vec(),
        )]);
        let t = now();
        let token = codec
            .mint(Uuid::new_v4(), vec![Role::Viewer], t, t + 900)
            .expect("mint");
        let header = decode_header(&token).expect("decode_header");
        assert_eq!(header.kid.as_deref(), Some("current"));
        assert_eq!(header.alg, Algorithm::HS256);
    }

    #[test]
    fn mint_rejects_excessive_ttl() {
        let codec = JwtCodec::test_codec();
        let t = now();
        let err = codec
            .mint(
                Uuid::new_v4(),
                vec![Role::Viewer],
                t,
                t + MAX_ACCESS_TTL_SECS + 1,
            )
            .unwrap_err();
        assert!(err.to_string().contains("exceeds max"), "{err}");
    }

    #[test]
    fn mint_rejects_exp_before_iat() {
        // M2/L3: exp < iat is rejected with Error::Invalid.
        let codec = JwtCodec::test_codec();
        let t = now();
        let err = codec
            .mint(Uuid::new_v4(), vec![Role::Viewer], t, t - 1)
            .unwrap_err();
        assert!(err.to_string().contains("exp must be >= iat"), "{err}");
    }

    #[test]
    fn mint_identity_roundtrip_carries_the_go_claim_set() {
        let codec = JwtCodec::test_codec();
        let t = now();
        let expiry = t + 900;
        let identity = Identity {
            user_id: Uuid::new_v4(),
            email: Some("ada@ex.com".into()),
            role: Some("USER".into()),
            user_type: Some("COMPANY".into()),
            jti: "session-id-as-jti".into(),
            roles: vec![Role::Viewer],
        };
        let token = codec.mint_identity(&identity, t, expiry).expect("mint");
        let claims = codec.verify(&token).expect("verify");
        assert_eq!(claims.sub, identity.user_id);
        assert_eq!(
            claims.email.as_deref(),
            Some("ada@ex.com"),
            "go email claim"
        );
        assert_eq!(claims.role.as_deref(), Some("USER"), "go DB role string");
        assert_eq!(claims.user_type.as_deref(), Some("COMPANY"));
        assert_eq!(claims.roles, [Role::Viewer], "rust RBAC snapshot rides too");
        assert_eq!(
            claims.jti, "session-id-as-jti",
            "jti == session id (go parity)"
        );
        assert_eq!(claims.iss, ACCESS_ISSUER);
        assert_eq!(claims.aud, ACCESS_AUDIENCE);
    }

    #[test]
    fn mint_identity_stamps_the_session_id_as_jti() {
        // go parity: jti is the session id — deactivating the session must
        // revoke the access token.
        let store = Arc::new(InMemorySessionStore::new());
        let codec = JwtCodec::test_codec().with_store(store.clone());
        let t = now();
        let identity = Identity {
            user_id: Uuid::new_v4(),
            email: None,
            role: Some("MODERATOR".into()),
            user_type: Some("USER".into()),
            jti: "sess-7".into(),
            roles: vec![Role::Editor],
        };
        let token = codec.mint_identity(&identity, t, t + 900).expect("mint");
        let claims = codec.verify(&token).expect("verify");
        assert_eq!(claims.jti, "sess-7");
        store.revoke("sess-7", claims.exp).expect("revoke");
        assert!(
            codec.verify(&token).is_err(),
            "deactivating the session revokes the access token"
        );
    }

    #[test]
    fn mint_identity_enforces_the_same_ttl_ceiling() {
        let codec = JwtCodec::test_codec();
        let t = now();
        let err = codec
            .mint_identity(
                &Identity {
                    user_id: Uuid::new_v4(),
                    email: None,
                    role: None,
                    user_type: None,
                    jti: "sess-8".into(),
                    roles: vec![Role::Viewer],
                },
                t,
                t + MAX_ACCESS_TTL_SECS + 1,
            )
            .unwrap_err();
        assert!(err.to_string().contains("exceeds max"), "{err}");
    }

    #[test]
    #[should_panic(expected = "duplicate kid")]
    fn new_rejects_duplicate_kid() {
        // L1: a duplicate kid would silently shadow a key during collect().
        let secret = b"test-secret-key-at-least-32-bytes!!".to_vec();
        let _ = JwtCodec::new(vec![
            ("dup".to_string(), secret.clone()),
            ("dup".to_string(), secret),
        ]);
    }

    #[test]
    fn verify_rejects_expired_token() {
        let codec = JwtCodec::test_codec();
        let t = now();
        // TTL within the cap, but exp is in the past → expired.
        let token = codec
            .mint(Uuid::new_v4(), vec![Role::Viewer], t - 400, t - 100)
            .expect("mint");
        assert!(codec.verify(&token).is_err(), "expired token must fail");
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        let codec_a = JwtCodec::from_single(b"secret-a-at-least-32-bytes-long!!");
        let codec_b = JwtCodec::from_single(b"secret-b-at-least-32-bytes-long!!");
        let t = now();
        let token = codec_a
            .mint(Uuid::new_v4(), vec![Role::Admin], t, t + 900)
            .expect("mint");
        assert!(
            codec_b.verify(&token).is_err(),
            "token signed with a different key must fail"
        );
    }

    #[test]
    fn verify_rejects_garbage() {
        let codec = JwtCodec::test_codec();
        assert!(codec.verify("not-a-jwt").is_err());
        assert!(codec.verify("").is_err());
    }

    #[test]
    fn verify_rejects_unknown_kid() {
        // Same secret, different kid labels: codec_b cannot resolve the kid
        // codec_a stamped, so it rejects even though the signature would match.
        let secret = b"shared-secret-at-least-32-bytes!!";
        let codec_a = JwtCodec::new(vec![("kid-a".to_string(), secret.to_vec())]);
        let codec_b = JwtCodec::new(vec![("kid-b".to_string(), secret.to_vec())]);
        let t = now();
        let token = codec_a
            .mint(Uuid::new_v4(), vec![Role::Viewer], t, t + 900)
            .expect("mint");
        assert!(
            codec_b.verify(&token).is_err(),
            "token with an unknown kid must fail"
        );
    }

    #[test]
    fn verify_rejects_token_without_kid() {
        let codec = JwtCodec::test_codec();
        let t = now();
        let claims = Claims {
            sub: Uuid::new_v4(),
            roles: vec![Role::Viewer],
            email: None,
            role: None,
            user_type: None,
            aud: ACCESS_AUDIENCE.to_string(),
            iss: ACCESS_ISSUER.to_string(),
            iat: t,
            exp: t + 900,
            jti: Uuid::new_v4().to_string(),
        };
        // Header with no kid — verify must reject before key lookup.
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"test-secret-key-at-least-32-bytes!!"),
        )
        .expect("encode");
        assert!(codec.verify(&token).is_err(), "token without kid must fail");
    }

    #[test]
    fn verify_rejects_non_hs256_alg() {
        let codec = JwtCodec::test_codec();
        let t = now();
        let claims = Claims {
            sub: Uuid::new_v4(),
            roles: vec![Role::Viewer],
            email: None,
            role: None,
            user_type: None,
            aud: ACCESS_AUDIENCE.to_string(),
            iss: ACCESS_ISSUER.to_string(),
            iat: t,
            exp: t + 900,
            jti: Uuid::new_v4().to_string(),
        };
        let mut header = Header::new(Algorithm::HS512);
        header.kid = Some("default".to_string());
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_secret(b"test-secret-key-at-least-32-bytes!!"),
        )
        .expect("encode");
        assert!(
            codec.verify(&token).is_err(),
            "non-HS256 alg must fail (no alg confusion)"
        );
    }

    #[test]
    fn verify_rejects_revoked_jti() {
        let store = Arc::new(InMemorySessionStore::new());
        let codec = JwtCodec::test_codec().with_store(store.clone());
        let sub = Uuid::new_v4();
        let t = now();
        let token = codec
            .mint(sub, vec![Role::Owner], t, t + 900)
            .expect("mint");
        // First verify succeeds and reveals the jti.
        let claims = codec.verify(&token).expect("verify before revoke");
        store.revoke(&claims.jti, claims.exp).expect("revoke");
        assert!(
            codec.verify(&token).is_err(),
            "revoked jti must fail verify"
        );
    }

    #[test]
    fn verify_rejects_excessive_ttl_bypassing_mint() {
        // H1: a token with iat=0, exp=now+10y signed with the CORRECT key
        // bypasses the mint-side TTL check (constructed via encode directly).
        // verify MUST re-assert the ceiling — this is the core H1 fix.
        let secret = b"test-secret-key-at-least-32-bytes!!";
        let codec = JwtCodec::from_single(secret);
        let t = now();
        let ten_years_secs = 365 * 10 * 86_400;
        let claims = Claims {
            sub: Uuid::new_v4(),
            roles: vec![Role::Owner],
            email: None,
            role: None,
            user_type: None,
            aud: ACCESS_AUDIENCE.to_string(),
            iss: ACCESS_ISSUER.to_string(),
            iat: 0,
            exp: t + ten_years_secs,
            jti: Uuid::new_v4().to_string(),
        };
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("default".to_string());
        let token = encode(&header, &claims, &EncodingKey::from_secret(secret)).expect("encode");
        assert!(
            codec.verify(&token).is_err(),
            "token with TTL > MAX_ACCESS_TTL_SECS must fail verify (H1)"
        );
    }

    #[test]
    fn verify_rejects_future_iat() {
        // H1: a token with iat in the future is suspicious — reject.
        let codec = JwtCodec::test_codec();
        let t = now();
        let claims = Claims {
            sub: Uuid::new_v4(),
            roles: vec![Role::Viewer],
            email: None,
            role: None,
            user_type: None,
            aud: ACCESS_AUDIENCE.to_string(),
            iss: ACCESS_ISSUER.to_string(),
            iat: t + 60,
            exp: t + 60 + 900,
            jti: Uuid::new_v4().to_string(),
        };
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("default".to_string());
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_secret(b"test-secret-key-at-least-32-bytes!!"),
        )
        .expect("encode");
        assert!(
            codec.verify(&token).is_err(),
            "token with future iat must fail verify (H1)"
        );
    }

    #[test]
    fn verify_rejects_oversized_jti() {
        // L4: jti > 64 chars is suspicious (a UUID is 36) — reject.
        let codec = JwtCodec::test_codec();
        let t = now();
        let claims = Claims {
            sub: Uuid::new_v4(),
            roles: vec![Role::Viewer],
            email: None,
            role: None,
            user_type: None,
            aud: ACCESS_AUDIENCE.to_string(),
            iss: ACCESS_ISSUER.to_string(),
            iat: t,
            exp: t + 900,
            jti: "x".repeat(65),
        };
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("default".to_string());
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_secret(b"test-secret-key-at-least-32-bytes!!"),
        )
        .expect("encode");
        assert!(
            codec.verify(&token).is_err(),
            "token with oversized jti must fail verify (L4)"
        );
    }

    #[test]
    fn extract_bearer_is_case_insensitive() {
        // L2: Bearer, bearer, BEARER are all valid per RFC 7235.
        use axum::http::HeaderMap;
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, "bearer abc".parse().unwrap());
        assert_eq!(extract_bearer(&h).as_deref(), Some("abc"));
        h.insert(AUTHORIZATION, "BEARER xyz".parse().unwrap());
        assert_eq!(extract_bearer(&h).as_deref(), Some("xyz"));
    }

    #[test]
    fn in_memory_store_prunes_expired_revocations() {
        // M1: a revoked jti whose token has expired (exp + leeway < now) is
        // evicted on is_revoked — the set stays bounded.
        let store = InMemorySessionStore::new();
        // Revoke a jti with exp far in the past.
        store.revoke("old-jti", 0).expect("revoke");
        // The entry is pruned on the next is_revoked call, so is_revoked
        // returns false (the token can't pass verify anyway — exp is past).
        assert!(
            !store.is_revoked("old-jti"),
            "expired revocation entry must be pruned"
        );
    }

    #[test]
    fn constants_have_no_placeholder_after_rename() {
        // L9: after rename.sh runs, the audience/issuer constants must not
        // contain literal <app>/<APP> placeholders. This catches a rename.sh
        // skip or a missed edit. We use unicode escapes (\u{3c} = '<',
        // \u{3e} = '>') so rename.sh's sed does NOT rewrite the placeholder
        // string inside this test — the check stays meaningful post-rename.
        let app_placeholder = "\u{3c}app\u{3e}";
        let app_upper_placeholder = "\u{3c}APP\u{3e}";
        assert!(
            !ACCESS_AUDIENCE.contains(app_placeholder),
            "ACCESS_AUDIENCE still has <app> placeholder — rename.sh missed it"
        );
        assert!(
            !ACCESS_AUDIENCE.contains(app_upper_placeholder),
            "ACCESS_AUDIENCE still has <APP> placeholder — rename.sh missed it"
        );
        assert!(
            !ACCESS_ISSUER.contains(app_placeholder),
            "ACCESS_ISSUER still has <app> placeholder — rename.sh missed it"
        );
        assert!(
            !ACCESS_ISSUER.contains(app_upper_placeholder),
            "ACCESS_ISSUER still has <APP> placeholder — rename.sh missed it"
        );
    }

    #[tokio::test]
    async fn middleware_attaches_principal_on_valid_token() {
        let codec = JwtCodec::test_codec();
        let sub = Uuid::new_v4();
        let t = now();
        let token = codec
            .mint(sub, vec![Role::Owner], t, t + 900)
            .expect("mint");
        let app = protected_app();
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/me")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), 256).await.unwrap();
        assert_eq!(&body[..], sub.to_string().as_bytes());
    }

    #[tokio::test]
    async fn middleware_rejects_missing_token() {
        let app = protected_app();
        let res = app
            .oneshot(Request::builder().uri("/me").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn middleware_rejects_malformed_header() {
        let app = protected_app();
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/me")
                    .header("authorization", "Bearer not-a-jwt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
