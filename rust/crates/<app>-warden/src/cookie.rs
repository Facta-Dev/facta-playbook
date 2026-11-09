//! Auth cookie helpers (go `internal/platform/authx/middleware.go`
//! `SetAuthCookie`/`ClearAuthCookie` parity): the access token rides an
//! `HttpOnly; SameSite=Strict` cookie (plus `Secure` outside dev), so an XSS
//! in the SPA cannot read the access token out of `document.cookie`.
//!
//! The bearer header stays the primary carrier ([`crate::extract_bearer`]);
//! the cookie is the fallback carrier — go's `bearerToken` checks the
//! `Authorization` header and falls back to the cookie.

use axum::http::header::HeaderMap;

/// Append a `Set-Cookie` establishing the auth cookie: `HttpOnly`,
/// `SameSite=Strict`, `Path=/`, `Secure` when `secure` is set
/// (go `SetAuthCookie` parity; go emits `Max-Age=0` for the clear).
///
/// The cookie name is caller-owned (the api composition root owns the
/// configured name); this helper only shapes the attributes.
pub fn set_auth_cookie(headers: &mut HeaderMap, name: &str, token: &str, secure: bool) {
    let mut value = format!("{name}={token}; Path=/; HttpOnly; SameSite=Strict");
    if secure {
        value.push_str("; Secure");
    }
    push_set_cookie(headers, &value);
}

/// Append a `Set-Cookie` clearing the auth cookie (go `ClearAuthCookie` with
/// `MaxAge: -1`, which emits an expired empty cookie).
pub fn clear_auth_cookie(headers: &mut HeaderMap, name: &str, secure: bool) {
    let mut value = format!("{name}=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict");
    if secure {
        value.push_str("; Secure");
    }
    push_set_cookie(headers, &value);
}

fn push_set_cookie(headers: &mut HeaderMap, cookie: &str) {
    headers.append(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(cookie)
            .expect("cookie values are controlled; header-safe"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_cookie_bears_the_secure_attributes() {
        let mut h = HeaderMap::new();
        set_auth_cookie(&mut h, "token", "jwt-value", true);
        let cookie = h
            .get(axum::http::header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .expect("cookie header");
        assert_eq!(
            cookie,
            "token=jwt-value; Path=/; HttpOnly; SameSite=Strict; Secure"
        );
    }

    #[test]
    fn set_cookie_without_secure_omits_the_flag() {
        let mut h = HeaderMap::new();
        set_auth_cookie(&mut h, "<app>_access_token", "jwt-value", false);
        let cookie = h
            .get(axum::http::header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .expect("cookie fallback carrier");
        assert_eq!(
            cookie,
            "<app>_access_token=jwt-value; Path=/; HttpOnly; SameSite=Strict"
        );
        assert!(!cookie.contains("Max-Age"), "set does not expire: {cookie}");
    }

    #[test]
    fn clear_cookie_uses_the_expired_empty_recipe() {
        let mut h = HeaderMap::new();
        clear_auth_cookie(&mut h, "token", false);
        let cookie = h
            .get(axum::http::header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .expect("cookie header");
        assert_eq!(
            cookie,
            "token=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict"
        );
    }

    #[test]
    fn successive_cookies_are_appended_not_replaced() {
        let mut h = HeaderMap::new();
        set_auth_cookie(&mut h, "a", "1", false);
        clear_auth_cookie(&mut h, "a", false);
        assert_eq!(
            h.get_all(axum::http::header::SET_COOKIE).iter().count(),
            2,
            "append keeps both Set-Cookie headers"
        );
    }
}
