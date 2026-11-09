//! Google One-Tap ID-token verification (go
//! `internal/platform/oauthx/google.go` parity): RS256 against Google's
//! published JWKS with max-age caching, `aud == client_id`, both issuer
//! spellings, expiry, and a verified email.
//!
//! Offline-guarded: `offline` refuses before any socket with
//! [`Error::Upstream`]. The key-set fetch sits behind the reqwest seam
//! ([`GoogleVerifier::refresh_keys`]) unit tests never hit — tests seed the
//! cache directly ([`GoogleVerifier::seed_jwks`], test-only) and mint RS256
//! tokens against a fixed fixture key, so the full verify path (header,
//! kid lookup, RS256 signature, aud/iss/email checks) runs with no network.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use <app>_core::{Error, Result};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;

/// Google's public signing-key set (go `googleJWKSURL`).
const GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";

/// Cache-Control fallback (go `maxAge` fallback): 5 minutes.
const FALLBACK_FRESHNESS: Duration = Duration::from_secs(300);

/// Signature/leeway applied to `exp` (go `jwt.WithLeeway(time.Minute)`).
const ID_TOKEN_LEEWAY_SECS: u64 = 60;

/// The verified bearer of a One-Tap credential (go `oauthx.GoogleIdentity`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleIdentity {
    pub sub: String,
    pub email: String,
    pub given_name: String,
    pub family_name: String,
    pub picture: String,
}

/// The subset of the Google ID token this slice consumes (go
/// `oauthx.googleClaims`). Missing fields are refused by deserialization.
#[derive(Debug, Deserialize)]
pub(crate) struct GoogleTokenClaims {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub iss: String,
    #[serde(default)]
    pub given_name: Option<String>,
    #[serde(default)]
    pub family_name: Option<String>,
    #[serde(default)]
    pub picture: Option<String>,
}

#[derive(Default)]
// DecodingKey has no Debug in jsonwebtoken 9 — map the keys count instead of
// leaking or hand-rolling key material in a derive.
struct JwksCache {
    keys: HashMap<String, DecodingKey>,
    fresh_until: Option<SystemTime>,
}

impl std::fmt::Debug for JwksCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwksCache")
            .field("keys", &self.keys.len())
            .field("fresh_until", &self.fresh_until)
            .finish()
    }
}

/// Verifies One-Tap credentials — RS256 over the cached Google JWKS (go
/// `oauthx.GoogleVerifier`). Cheap to clone.
#[derive(Clone)]
pub struct GoogleVerifier {
    jwks_url: String,
    offline: bool,
    cache: std::sync::Arc<Mutex<JwksCache>>,
}

impl std::fmt::Debug for GoogleVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the cached key material.
        f.debug_struct("GoogleVerifier")
            .field("jwks_url", &self.jwks_url)
            .field("offline", &self.offline)
            .finish_non_exhaustive()
    }
}

impl GoogleVerifier {
    /// Build the verifier; an empty `jwks_url` selects Google's certs.
    #[must_use]
    pub fn new(jwks_url: &str, offline: bool) -> Self {
        let url = if jwks_url.is_empty() {
            GOOGLE_JWKS_URL.to_owned()
        } else {
            jwks_url.to_owned()
        };
        Self {
            jwks_url: url,
            offline,
            cache: std::sync::Arc::new(Mutex::new(JwksCache::default())),
        }
    }

    /// Verify the RS256 ID token: signature (kid from the max-age-cached
    /// JWKS), audience, issuer (both Google spellings), expiry, and a
    /// verified email.
    ///
    /// # Errors
    /// - [`Error::Upstream`] on an offline refusal (`offline mode forbids
    ///   network calls`) or a JWKS fetch/parse failure.
    /// - [`Error::Unauthenticated`] ("invalid google token") on any
    ///   signature/claims refusal.
    pub async fn verify(&self, credential: &str, client_id: &str) -> Result<GoogleIdentity> {
        if self.offline {
            return Err(Error::Upstream {
                service: "google",
                detail: "offline mode forbids network calls".into(),
            });
        }
        let header = decode_header(credential).map_err(|_| bad_id_token())?;
        if header.alg != Algorithm::RS256 {
            return Err(bad_id_token());
        }
        let Some(kid) = header.kid else {
            return Err(bad_id_token());
        };
        let key = self.key_for(&kid).await?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[client_id]);
        validation.leeway = ID_TOKEN_LEEWAY_SECS;
        let data = decode::<GoogleTokenClaims>(credential, &key, &validation)
            .map_err(|_| bad_id_token())?;
        validate_claims(&data.claims)
    }

    /// Resolve the token's `kid` from the cache, refreshing the JWKS when
    /// stale or unknown (Google rotates keys; an unknown `kid` forces one
    /// bypass of the cache before giving up — go `publicKey` parity).
    async fn key_for(&self, kid: &str) -> Result<DecodingKey> {
        let stale = {
            let cache = self.lock();
            cache
                .fresh_until
                .is_none_or(|until| SystemTime::now() > until)
        };
        if stale {
            // First fetch must succeed when no usable set is cached; a stale
            // but non-empty set may still verify (go parity).
            if let Err(err) = self.refresh_keys().await {
                let non_empty = {
                    let cache = self.lock();
                    !cache.keys.is_empty()
                };
                if !non_empty {
                    return Err(err);
                }
            }
        }
        if let Some(key) = self.lock().keys.get(kid) {
            return Ok(key.clone());
        }
        // Unknown kid: exactly one forced refresh, then give up.
        self.refresh_keys().await?;
        self.lock().keys.get(kid).cloned().ok_or_else(bad_id_token)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, JwksCache> {
        self.cache.lock().expect("jwks cache lock poisoned")
    }

    /// Refetch the JWKS and honor `Cache-Control: max-age` (go
    /// `refreshLocked`). Builds a throwaway reqwest client per refresh, like
    /// go builds one per fetch. Never reached while `offline` (checked by
    /// `verify`) and never reached in unit tests (cache is seeded).
    async fn refresh_keys(&self) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| Error::Upstream {
                service: "google",
                detail: format!("jwks client: {e}"),
            })?;
        let resp = client
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|e| Error::Upstream {
                service: "google",
                detail: format!("jwks fetch: {e}"),
            })?;
        let status = resp.status();
        let cache_control = resp
            .headers()
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        let body = resp.text().await.map_err(|e| Error::Upstream {
            service: "google",
            detail: format!("jwks read: {e}"),
        })?;
        if !status.is_success() {
            return Err(Error::Upstream {
                service: "google",
                detail: format!("jwks fetch: status {status}"),
            });
        }
        let keys = parse_jwks(&body)?;
        // Single-flight-free update: hold the (std) lock only for the swap;
        // concurrent verifiers may each fetch (rare — Google rotates slowly).
        let fresh_until = SystemTime::now() + max_age(&cache_control);
        drop(body);
        let mut cache = self.lock();
        cache.keys = keys.into_iter().collect();
        cache.fresh_until = Some(fresh_until);
        drop(cache);
        Ok(())
    }

    /// Test-only: seed the cache as if a JWKS fetch had just succeeded, so
    /// `verify` runs its full offline path (signature, aud, iss, email)
    /// without ever opening a socket.
    #[cfg(test)]
    fn seed_jwks(&self, body: &str) {
        let mut cache = self.lock();
        cache.keys = parse_jwks(body)
            .expect("test jwks body must parse")
            .into_iter()
            .collect();
        cache.fresh_until = Some(SystemTime::now() + Duration::from_secs(3600));
    }
}

fn bad_id_token() -> Error {
    Error::Unauthenticated("invalid google token".into())
}

/// The claims-level checks of `verify`, factored out for offline testing
/// (go mirrors this inside `Verify`): both Google issuer spellings, then a
/// non-empty subject and a verified email.
fn validate_claims(c: &GoogleTokenClaims) -> Result<GoogleIdentity> {
    if c.iss != "accounts.google.com" && c.iss != "https://accounts.google.com" {
        return Err(bad_id_token());
    }
    if c.sub.is_empty() || c.email.is_empty() || !c.email_verified {
        return Err(bad_id_token());
    }
    Ok(GoogleIdentity {
        sub: c.sub.clone(),
        email: c.email.clone(),
        given_name: c.given_name.clone().unwrap_or_default(),
        family_name: c.family_name.clone().unwrap_or_default(),
        picture: c.picture.clone().unwrap_or_default(),
    })
}

/// One RSA entry of the published key set (go `oauthx.jwksKey`).
#[derive(Debug, Deserialize)]
struct JwksKey {
    kid: Option<String>,
    kty: Option<String>,
    n: Option<String>,
    e: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JwksDoc {
    keys: Vec<JwksKey>,
}

/// Parse a JWKS body into `(kid, DecodingKey)` pairs, skipping non-RSA or
/// malformed entries (go `refreshLocked` parity); an empty usable set is a
/// parse-style [`Error::Upstream`].
pub(crate) fn parse_jwks(body: &str) -> Result<Vec<(String, DecodingKey)>> {
    let doc: JwksDoc = serde_json::from_str(body).map_err(|e| Error::Upstream {
        service: "google",
        detail: format!("jwks parse: {e}"),
    })?;
    let mut keys = Vec::new();
    for k in doc.keys {
        if k.kty.as_deref() != Some("RSA") {
            continue;
        }
        let (Some(kid), Some(n), Some(e)) = (k.kid, k.n, k.e) else {
            continue;
        };
        if let Ok(key) = DecodingKey::from_rsa_components(&n, &e) {
            keys.push((kid, key));
        } // malformed n/e entries are skipped (go parity)
    }
    if keys.is_empty() {
        return Err(Error::Upstream {
            service: "google",
            detail: "jwks has no rsa keys".into(),
        });
    }
    Ok(keys)
}

/// Parse `max-age=N` out of `Cache-Control`; a missing or malformed value
/// falls back to 5 minutes (go `maxAge` parity).
#[must_use]
pub(crate) fn max_age(cache_control: &str) -> Duration {
    for dir in cache_control.split(',') {
        let dir = dir.trim();
        if let Some(value) = dir.strip_prefix("max-age=") {
            return value
                .parse::<u64>()
                .map(Duration::from_secs)
                .unwrap_or(FALLBACK_FRESHNESS);
        }
    }
    FALLBACK_FRESHNESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde::Serialize;

    /// Fixture RSA key: a throwaway 2048-bit key generated for these tests
    /// ONLY (never a production credential). The JWKS body publishes its
    /// public half under `kid = "k1"`.
    const FIXTURE_PEM: &str = "-----BEGIN PRIVATE KEY-----\n\
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQDXwDSw8iT39ISM\n\
KjLuR0jt1yF6h2v1Aetb8yPwvOluVtZzC/9eQhtVL+hsYXrbo7W4b4jmQ7WJC/hp\n\
D8EqEq4GuolxSuQjAQhUjQ0VGhms7jBop4mQ4wWoBKSTcTEZTJtlWMRAcqF6zxZV\n\
/Eke+Grx/fF3qc5bUwLqGlwqPrzYA2CaHoT5Di6WWINAIDTAlu+r3MhVZy3LCDcP\n\
K1cdpKWbp3XS3JVXDVJNa/tl9mjwi8r++IaP+wswuFgpoTFDth7EI6iu3zlht6li\n\
lWE+h8Mvovag29vL5y30YOQkpS7loWT8sszcFDe+Wx8TKi4HNwbQDyMTTVoEXGO/\n\
X8cdT/phAgMBAAECggEADEY+mSyFhufN3K2P7Td4AjQBjxeCQFOmILqR+mG9BA6D\n\
r9LoaePn6y5JElVvaGyFa6NIzhTfEJdyQAPp8OtUJETbiesBVK5721eU/XTZLhn0\n\
/hc9/9Syny74b/z6Fp7hnl/w9luvLS337tdmD5GBgBf681xhRZfQ7CGoyax9EOrx\n\
h5t+siTqJdnQ4Bz+NydudAGVk01qmmAQae5GSDGStp2BgqtqQy2DHuw2A4Kr1DyK\n\
Z+EOfFCaqH7TURk1cl4TSBfDVobQF1OR482SOFTDZy9BmRXwUBDZO4JUsxdvoFKi\n\
GI9HrDTDWUGzdy/fOI1549v1FgThIm6iaG+Z/PpuywKBgQDsp3aBnsoJxmoZa6KS\n\
STxMmQxB1fK1tf7qD7+ndLizrA0csVLmeiDhNw3xgTEQzvmhcsfweLPpFBUfKIIa\n\
gzsAs5m75sFn4xJ3MEyAZZCPzh3AKI7eF6HDIS2tTdSO4r8sRMlTw9Iu7m/ON5oD\n\
jHdg5lmMpVHZSqcKfY0lmG5xswKBgQDpY0q3nFgamRL+anQ0okp+lbZ/XW8wT+at\n\
FpSgoBvXjrVJ6SNz19Lp9+TJ9Kct17xYI59p0Fh4yecsyb7BXhbULK+6KL7aYGx3\n\
wg/jf2Y+IT5NNRRsAbV8GlPqoamcbFP/Th1u1LwknkY9vlMzr2XGknyjPIPaB/Zz\n\
GleqXurRmwKBgApsyhylqRLLKcta4Xt+PGjHaTHRQfLdRJB9Bx1SV/yaPK2h5hv/\n\
D0m8iiG0ZjvuUjLZgpNGTe6bkt6fgzO1yjiasNAFN0njR+H2P7h8QOUE2GypnLmN\n\
4vNcl82FmzJ8LJ2/nfrRqoWkvUohSuiTHIMpQpKeib9/uWIN69TXJS5fAoGAD7gO\n\
SjuPe4sfyl7lIL5lgg0XU5C5cmmPQc0piX//IA+OikHXj2/T1spJcIi30dCb8NvF\n\
Wt0beFcw6MVVwbRSmUvBMiqYYCDqbU52v33PJxQaHB0dGQEsdgVX7Y+h2tU4+xMJ\n\
zHMrkNPQK0xsUijYE/29w0jKIJ80rB/0E5d8/U0CgYA+UhgBjV1/bboyCR0DXTrL\n\
BtKeobo0jqbOE9XL/BZs29QUzyvpL+Z5zaIhzBagMoTT0Nt5RJCFaIzwpaYEOCjH\n\
3oFW7qGDKtFgTzXy1T9Bw9GC2he0xIio5UpIlN5DCDPs+Aq4LEzTQ2sGdClcusgG\n\
0CG+mVGPqsT/AXEZbiAUBA==\n\
-----END PRIVATE KEY-----\n\
";

    const FIXTURE_N_B64URL: &str = "18A0sPIk9_SEjCoy7kdI7dcheodr9QHrW_Mj8LzpblbWcwv_XkIbVS_obGF626O1uG-I5kO1iQv4aQ_BKhKuBrqJcUrkIwEIVI0NFRoZrO4waKeJkOMFqASkk3ExGUybZVjEQHKhes8WVfxJHvhq8f3xd6nOW1MC6hpcKj682ANgmh6E-Q4ulliDQCA0wJbvq9zIVWctywg3DytXHaSlm6d10tyVVw1STWv7ZfZo8IvK_viGj_sLMLhYKaExQ7YexCOort85YbepYpVhPofDL6L2oNvby-ct9GDkJKUu5aFk_LLM3BQ3vlsfEyouBzcG0A8jE01aBFxjv1_HHU_6YQ";

    // A second throwaway test-only RSA private key, NOT in the seeded cache, sent
    // under the same kid in the signed-by-another-key test (its signature must
    // fail). Test-only material, never production.
    const FIXTURE_PEM2: &str = "-----BEGIN PRIVATE KEY-----\n\
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC0AOXEVBWMcu3B\n\
qbdPqR5eOFhskFPha/XL5EIu1mhlBKuBWS8yROb2mzxVqqeQxD75TTncZANoih0Q\n\
DP15ILGm5O8FxvxvlfnXSwzEJgNQOjB4RHtUVXm1i1761eEPfJe20zpsVnAGWxoj\n\
kU2L+tkact61GGuAiWS6VSLTSiToLUYmxrANMbgzRcToOVA5WQoBaixGRqZEAjhr\n\
jwk+b42X1dJIp9XrWXld/OkpRyhzNvOPYfg34iVxzaepDxhojcKYwJ7YjpOCP6ki\n\
0m+UZ72Ea7EPm1Q4x7uHcRU4f3L58vgxK3DqBwCGFGmMagv5Uy96izftHZNg2MtT\n\
Nz4REBB5AgMBAAECggEAKMFdRJb/LAYnk+Yv1Y8Gi7G5P5dsDomJpQkA/SbZiynE\n\
BSzitUCJhMOAJStW9j2gvoSAOcjBRs2ip+wxBdhjsNR70/SKWl98KoVZFh4/OM58\n\
A/cJb35M6NBuCGAdrYp5tVDdFQN7KM0+vVfcVXnN5yGkEB+/02IIVvIIyQssEQ8x\n\
ESqd+bpezSM4gj7crp+m8Se5FsFhFghLHBu2KV13dvPvyeWTvpo0Jz4c08Zop0fN\n\
qHD8ypNfBHNLlNj3btnO1m7hbvHlvkIDvpoDz5GDNEyZV5w9r5jAC2dWoRXYx+jA\n\
sKzBgbI63pa2Sac0q8VqC+p9HqBNiZsmWxXjVoVuKQKBgQD1jJEUjomTow21whBe\n\
GH6lDq6/M35OTWK/Ikh9Nwm34Jl2Maq3rXupabiZIX+Q8v1XF3rErtrRqSfLofBu\n\
AVb3DKyY+ZPbT0N5NVZ3hi2VduyWrhzf9AizntVUOxJUxItQnwL1w9WG0sJeOp0m\n\
2AetD+g9Fkg1rv5/l1fim6AshQKBgQC7qioqpjyMMJsPRTmjhsztfLSbblVF0Lrn\n\
FRKlpbz/OfnvCkapqfrIiVuPHaSdHzTyC3XmkjZ1k2JgSEo7VdP8vQCXI65xfv6r\n\
6QC16IrVKq6uyoirzo9D4BirX71TqSsE7M2ipAuqY9HnbZ6404UD0AdLHNqOX77H\n\
ZShaguyAZQKBgDDHUs7NngDpJEEWIyA+J3Jsn0Q2W4H8l4A2tEXDmDeh83pqVWqV\n\
mPpHmTc6U1gdHTye2nGV9rwtqU0nh1ajB36eUu80rS2k6ld6xAmMayPE8yzfu7a1\n\
iMD97Y0FCxgxzycd332pAJL+7UPAyxU7VxMHFPCFq6SWddIs43WNytztAoGAKQcy\n\
4NAnPJ8Dx3ri9xVpiCJTegfNLM5NjmfVA9IQV9xdAB3HBSDcaytckRVx/8fq02Ae\n\
DqK4tUvbniEv2XkSN5c6odBzOu0A0KTIOvz0HVaPijdNWgJ23+gGfLEZu0io1xLO\n\
glhLFJ1uLqKQqOfVlNfOQGW2gpn8I6vB/sfQ3OECgYEAnbQqwwZNog4nDZop4Q9C\n\
TPzgmDGO8+qrbzhoRv3Cla+y8DMILyQLiy0B2m72gKUcyVHs04CyaXfkTc7DxFUq\n\
P+R9QMPs8/IfeMa9IcTKpyNKHPOEiq4ZEEGWJVfITZArmTG9goiE2lT5oiC7caAi\n\
AOGKsVoKlPqqbN33Jhp6yCQ=\n\
-----END PRIVATE KEY-----\n";

    const GOOGLE_ISS: &str = "https://accounts.google.com";

    fn fixture_jwks(kid: &str) -> String {
        format!(r#"{{"keys":[{{"kty":"RSA","kid":"{kid}","n":"{FIXTURE_N_B64URL}","e":"AQAB"}}]}}"#)
    }

    fn verifier_with_fresh_cache(kid: &str) -> GoogleVerifier {
        let verifier = GoogleVerifier::new("https://unused.invalid/certs", false);
        verifier.seed_jwks(&fixture_jwks(kid));
        verifier
    }

    fn encoding_key() -> EncodingKey {
        EncodingKey::from_rsa_pem(FIXTURE_PEM.as_bytes()).expect("fixture pem")
    }

    #[derive(Serialize)]
    struct TestClaims {
        iss: &'static str,
        aud: &'static str,
        sub: &'static str,
        email: &'static str,
        email_verified: bool,
        exp: i64,
        iat: i64,
    }

    /// Mint an RS256 id token signed by the fixture key under `kid`.
    fn mint_test_token(
        kid: &str,
        edits: impl FnOnce(&mut TestClaims),
        alg_override: Option<Algorithm>,
    ) -> String {
        let now = crate::current_unix_secs();
        let mut claims = TestClaims {
            iss: GOOGLE_ISS,
            aud: "g-client",
            sub: "123",
            email: "a@ex.com",
            email_verified: true,
            exp: now + 3600,
            iat: now,
        };
        edits(&mut claims);
        let mut header = Header::new(alg_override.unwrap_or(Algorithm::RS256));
        header.kid = Some(kid.to_owned());
        encode(&header, &claims, &encoding_key()).expect("mint test token")
    }

    #[tokio::test]
    async fn verify_accepts_a_valid_id_token() {
        let verifier = verifier_with_fresh_cache("k1");
        let identity = verifier
            .verify(&mint_test_token("k1", |_| {}, None), "g-client")
            .await
            .expect("verify");
        assert_eq!(identity.sub, "123");
        assert_eq!(identity.email, "a@ex.com");
        assert_eq!(
            identity.given_name, "",
            "absent profile claims default empty"
        );
        assert_eq!(identity.family_name, "");
        assert_eq!(identity.picture, "");
    }

    #[tokio::test]
    async fn verify_maps_profile_claims() {
        #[derive(Serialize)]
        struct Rich {
            iss: &'static str,
            aud: &'static str,
            sub: &'static str,
            email: &'static str,
            email_verified: bool,
            exp: i64,
            #[serde(rename = "given_name")]
            given_name: &'static str,
            #[serde(rename = "family_name")]
            family_name: &'static str,
            #[serde(rename = "picture")]
            picture: &'static str,
        }
        let now = crate::current_unix_secs();
        let token = encode(
            &{
                let mut h = Header::new(Algorithm::RS256);
                h.kid = Some("k1".into());
                h
            },
            &Rich {
                iss: GOOGLE_ISS,
                aud: "g-client",
                sub: "123",
                email: "a@ex.com",
                email_verified: true,
                exp: now + 3600,
                given_name: "Ada",
                family_name: "Lovelace",
                picture: "pic",
            },
            &encoding_key(),
        )
        .expect("mint");
        let identity = verifier_with_fresh_cache("k1")
            .verify(&token, "g-client")
            .await
            .expect("verify");
        assert_eq!(
            (
                identity.given_name.as_str(),
                identity.family_name.as_str(),
                identity.picture.as_str()
            ),
            ("Ada", "Lovelace", "pic")
        );
    }

    #[tokio::test]
    async fn verify_rejects_wrong_audience() {
        let verifier = verifier_with_fresh_cache("k1");
        let token = mint_test_token("k1", |c| c.aud = "someone-else", None);
        let err = verifier.verify(&token, "g-client").await.unwrap_err();
        assert!(matches!(err, Error::Unauthenticated(_)), "{err}");
    }

    #[tokio::test]
    async fn verify_rejects_unknown_issuer() {
        let verifier = verifier_with_fresh_cache("k1");
        let token = mint_test_token("k1", |c| c.iss = "https://evil.example", None);
        let err = verifier.verify(&token, "g-client").await.unwrap_err();
        assert!(matches!(err, Error::Unauthenticated(_)), "{err}");
    }

    #[tokio::test]
    async fn verify_accepts_both_google_issuer_spellings() {
        let verifier = verifier_with_fresh_cache("k1");
        for iss in ["accounts.google.com", "https://accounts.google.com"] {
            let token = mint_test_token("k1", |c| c.iss = iss, None);
            verifier
                .verify(&token, "g-client")
                .await
                .unwrap_or_else(|e| panic!("iss {iss} must be accepted: {e}"));
        }
    }

    #[tokio::test]
    async fn verify_rejects_unverified_email() {
        let verifier = verifier_with_fresh_cache("k1");
        let token = mint_test_token("k1", |c| c.email_verified = false, None);
        assert!(verifier.verify(&token, "g-client").await.is_err());
    }

    #[tokio::test]
    async fn verify_rejects_hs256_alg_confusion() {
        // An HS256 token carrying the same kid must be refused by alg, not
        // signature (go WithValidMethods(["RS256"]) parity).
        let verifier = verifier_with_fresh_cache("k1");
        let now = crate::current_unix_secs();
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("k1".into());
        let token = encode(
            &header,
            &TestClaims {
                iss: GOOGLE_ISS,
                aud: "g-client",
                sub: "123",
                email: "a@ex.com",
                email_verified: true,
                exp: now + 3600,
                iat: now,
            },
            &EncodingKey::from_secret(b"hs256-secret-not-the-fixture"),
        )
        .expect("mint hs256 token");
        assert!(verifier.verify(&token, "g-client").await.is_err());
    }

    #[tokio::test]
    async fn verify_rejects_id_signed_by_another_key() {
        // A token whose signature does not verify against the fixture JWKS.
        let verifier = verifier_with_fresh_cache("k1");
        let other = EncodingKey::from_rsa_pem(FIXTURE_PEM2.as_bytes()).expect("fixture pem 2");
        let now = crate::current_unix_secs();
        let token = encode(
            &{
                let mut h = Header::new(Algorithm::RS256);
                h.kid = Some("k1".into());
                h
            },
            &TestClaims {
                iss: GOOGLE_ISS,
                aud: "g-client",
                sub: "123",
                email: "a@ex.com",
                email_verified: true,
                exp: now + 3600,
                iat: now,
            },
            &other,
        )
        .expect("mint");
        // RS256 decode against the fixture RSA key fails: wrong key family.
        assert!(verifier.verify(&token, "g-client").await.is_err());
    }

    #[tokio::test]
    async fn verify_refuses_offline_before_any_socket() {
        let verifier = GoogleVerifier::new("https://unused.invalid/certs", true);
        let err = verifier
            .verify(&mint_test_token("k1", |_| {}, None), "g-client")
            .await
            .unwrap_err();
        match &err {
            Error::Upstream { service, detail } if *service == "google" => {
                assert_eq!(detail, "offline mode forbids network calls", "{detail}");
            }
            other => panic!("offline refusal must be Upstream(google): {other:?}"),
        }
    }

    #[test]
    fn parse_jwks_skips_non_rsa_and_malformed_entries() {
        let body = format!(
            r#"{{"keys":[
                {{"kty":"oct","kid":"hmac-key","k":"AAAA"}},
                {{"kty":"RSA","kid":"bad-n-e","n":"!!!!not-b64url","e":"AQAB"}},
                {{"kty":"RSA","n":"{FIXTURE_N_B64URL}","e":"AQAB"}},
                {{"kty":"RSA","kid":"k1","n":"{FIXTURE_N_B64URL}","e":"AQAB"}}
            ]}}"#
        );
        let keys = parse_jwks(&body).expect("parse");
        assert_eq!(keys.len(), 1, "only the complete RSA entry survives");
        assert_eq!(keys[0].0, "k1");
    }

    #[test]
    fn parse_jwks_empty_and_garbage_are_errors() {
        assert!(parse_jwks(r#"{"keys":[]}"#).is_err(), "empty set refused");
        let Err(err) = parse_jwks("not json") else {
            panic!("garbage json must not parse");
        };
        assert!(
            matches!(
                err,
                Error::Upstream {
                    service: "google",
                    ..
                }
            ),
            "{err:?}"
        );
    }

    #[test]
    fn max_age_honors_and_falls_back() {
        assert_eq!(max_age("public, max-age=60"), Duration::from_secs(60));
        assert_eq!(max_age("max-age=3600"), Duration::from_secs(3600));
        assert_eq!(max_age("max-age=0"), Duration::from_secs(0));
        assert_eq!(
            max_age("max-age=-1"),
            FALLBACK_FRESHNESS,
            "negative → fallback"
        );
        assert_eq!(max_age("max-age=abc"), FALLBACK_FRESHNESS);
        assert_eq!(max_age("no-store"), FALLBACK_FRESHNESS);
        assert_eq!(max_age(""), FALLBACK_FRESHNESS);
    }
}
