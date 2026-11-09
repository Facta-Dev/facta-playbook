//! Identity service crate — the four go-parity use cases over the landed
//! [`<app>_ports`] contracts: signup (6-digit verification codes + profile
//! seeding), session (login/refresh with the retry lock and in-place rotation,
//! `jti == session id`), password (forgot/validate/reset/change) and oauth
//! (google one-tap + github oauth).
//!
//! Error surface: [`<app>_core::Error`] carries the status (go 422 business
//! validation reads as `Error::Invalid`, 400); the api layer renders the
//! envelope. Token minting rides the warden's `mint` gate — default-ON in
//! the api composition (go parity: login/refresh/oauth mint tokens); a
//! build that opts out keeps the routes but the mint shims answer 500.

// `pub` so the sibling modules' cross-module `crate::fakes::…` access makes
// the explicit item visibilities inside real (unreachable_pub parity); the
// module only exists in test builds and never exports in production.
#[cfg(test)]
pub mod fakes;

pub mod oauth;
pub mod password;
pub mod session;
pub mod signup;

pub use oauth::{LINK_STATE_PREFIX, OAuthService};
pub use password::PasswordService;
pub use session::{LoginInput, SessionService, SessionTokens};
pub use signup::{SignupInput, SignupService};
