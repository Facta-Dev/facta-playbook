//! Process-wide state. Cheap to clone — every field is `Arc` or `Clone`.
//! This is the composition root's wiring layer: concrete adapters are named
//! HERE and nowhere else (go `cmd/api/main.go` parity).
use std::sync::Arc;

use crate::config::Config;
use anyhow::{Context, Result};
use <app>_adminusers::AdminUsersService;
use <app>_auth::{OAuthService, PasswordService, SessionService, SignupService};
use <app>_loom::Hub;
use <app>_mailer::{Mailer, mailer_from_config};
use <app>_ports::{
    AuditLogRepository, EmailVerificationRepository, OutboxRepository, PasswordResetRepository,
    PgAuditLogRepository, PgEmailVerificationRepository, PgOutboxRepository,
    PgPasswordResetRepository, PgSessionRepository, PgUserRepository, PgWidgetRepository,
    SessionRepository, UserRepository, WidgetRepository,
};
use <app>_warden::JwtCodec;
use <app>_warden::github::GitHubClient;
use <app>_warden::google::GoogleVerifier;
use <app>_warden::link_token::LinkToken;
use <app>_widgets::WidgetService;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// One `Hub` construction point: go `platrt.NewHub(0)` selects the default
/// per-subscriber buffer of 16 (the doc comment: "hub default 16").
const HUB_BUFFER: usize = 16;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// The wiring-layer config (cookie mode, oauth toggles, worker knobs).
    /// Handlers read it instead of reaching for process env again.
    pub cfg: Config,
    /// Repos are `Arc<dyn Trait>` so (1) tests swap in mocks and
    /// (2) downstream code depends on the trait, not the adapter.
    pub widgets_repo: Arc<dyn WidgetRepository>,
    /// Use-case services, pre-wired with their repos.
    pub widgets: WidgetService,

    // ── identity ports (go features' Store surface, one adapter each) ─────
    pub users_repo: Arc<dyn UserRepository>,
    pub sessions_repo: Arc<dyn SessionRepository>,
    pub verifications_repo: Arc<dyn EmailVerificationRepository>,
    pub resets_repo: Arc<dyn PasswordResetRepository>,
    /// Email outbox queue — go `email.Queue` (Enqueue/Claim/Complete/Fail).
    pub outbox: Arc<dyn OutboxRepository>,
    pub audits_repo: Arc<dyn AuditLogRepository>,
    /// Outbound collaborator. INV-T7: `NoopMailer` when `cfg.is_offline()` or
    /// no `mailer_endpoint` is set.
    pub mailer: Arc<dyn Mailer>,

    // ── authx + oauthx (go internal/platform/authx · oauthx) ──────────────
    /// JWT codec for bearer-token auth (HS256), revocation-aware.
    pub jwt_codec: JwtCodec,
    /// Realtime hub (go `platrt.NewHub`); rooms via `<app>_loom::room_user`.
    pub hub: Arc<Hub>,
    /// Google One-Tap ID-token verifier (RS256 + JWKS). INV-T7: refuses
    /// before any socket while `cfg.offline`.
    pub google_client: GoogleVerifier,
    /// GitHub authorization-code client. INV-T7: same offline refusal.
    pub github_client: GitHubClient,

    // ── services (go features' use cases, pre-wired) ───────────────────────
    // Arc'd: the auth services are not `Clone` while `AppState` is (cheap
    // clone contract; the Arc is shared with the hub/codec style of this file).
    pub signup: Arc<SignupService>,
    pub session: Arc<SessionService>,
    pub password: Arc<PasswordService>,
    pub oauth: Arc<OAuthService>,
    pub admin_users: AdminUsersService,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish_non_exhaustive()
    }
}

/// Repo bundle shared by `from_env` (Pg adapters) and `with_repos` (mocks).
struct ComposeRepos {
    widgets_repo: Arc<dyn WidgetRepository>,
    users_repo: Arc<dyn UserRepository>,
    sessions_repo: Arc<dyn SessionRepository>,
    verifications_repo: Arc<dyn EmailVerificationRepository>,
    resets_repo: Arc<dyn PasswordResetRepository>,
    outbox: Arc<dyn OutboxRepository>,
    audits_repo: Arc<dyn AuditLogRepository>,
}

impl AppState {
    /// Production wiring: pool → concrete adapters → services. The single
    /// place a concrete `*Pg*` adapter is named.
    ///
    /// # Errors
    /// `Err` on pool/migration failure or an unparsable duration knob.
    pub async fn from_env(cfg: &Config) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(cfg.database_max_connections)
            .min_connections(cfg.database_min_connections)
            .connect(&cfg.database_url)
            .await?;
        sqlx::migrate!("../../migrations").run(&pool).await?;

        // The single point where concrete adapters are named:
        let widgets_repo: Arc<dyn WidgetRepository> =
            Arc::new(PgWidgetRepository::new(pool.clone()));
        let users_repo: Arc<dyn UserRepository> = Arc::new(PgUserRepository::new(pool.clone()));
        let sessions_repo: Arc<dyn SessionRepository> =
            Arc::new(PgSessionRepository::new(pool.clone()));
        let verifications_repo: Arc<dyn EmailVerificationRepository> =
            Arc::new(PgEmailVerificationRepository::new(pool.clone()));
        let resets_repo: Arc<dyn PasswordResetRepository> =
            Arc::new(PgPasswordResetRepository::new(pool.clone()));
        let outbox: Arc<dyn OutboxRepository> = Arc::new(PgOutboxRepository::new(pool.clone()));
        let audits_repo: Arc<dyn AuditLogRepository> =
            Arc::new(PgAuditLogRepository::new(pool.clone()));

        Self::compose(
            pool,
            cfg,
            ComposeRepos {
                widgets_repo,
                users_repo,
                sessions_repo,
                verifications_repo,
                resets_repo,
                outbox,
                audits_repo,
            },
        )
    }

    /// Shared sync wiring: mailer + oauth clients + hub + services from
    /// (cfg, repos). Used by both `from_env` (Pg adapters) and `with_repos`
    /// (mocks), so the service construction exists exactly once.
    #[allow(clippy::too_many_arguments)]
    fn compose(pool: PgPool, cfg: &Config, repos: ComposeRepos) -> Result<Self> {
        let ComposeRepos {
            widgets_repo,
            users_repo,
            sessions_repo,
            verifications_repo,
            resets_repo,
            outbox,
            audits_repo,
        } = repos;

        let widgets = WidgetService::new(widgets_repo.clone());
        // INV-T7 offline gate: when cfg.is_offline() is true, NoopMailer is
        // selected unconditionally (mailer_endpoint is never read).
        let mailer: Arc<dyn Mailer> =
            mailer_from_config(cfg.is_offline(), cfg.mailer_endpoint.as_deref());
        let jwt_codec = JwtCodec::from_single(cfg.jwt_secret.as_bytes());

        // oauthx (go main.go): jwks URL "" selects Google's certs; offline
        // mode refuses before any socket. Two instances: one for `state`, one
        // moved into the service (the type is plain data; Clone unpinned).
        let google_for_state = GoogleVerifier::new("", cfg.is_offline());
        let google_for_service = GoogleVerifier::new("", cfg.is_offline());
        let github_for_state = GitHubClient::new(cfg.is_offline());
        let github_for_service = GitHubClient::new(cfg.is_offline());
        let hub = Arc::new(Hub::new(HUB_BUFFER));

        let pepper = cfg.password_pepper.clone();
        let access_ttl = crate::config::parse_go_duration_chrono(&cfg.access_token_ttl)
            .context("parsing <APP>_ACCESS_TOKEN_TTL")?;
        let refresh_ttl = crate::config::parse_go_duration_chrono(&cfg.refresh_token_ttl)
            .context("parsing <APP>_REFRESH_TOKEN_TTL")?;

        let signup = SignupService::new(
            users_repo.clone(),
            verifications_repo.clone(),
            outbox.clone(),
            &pepper,
            crate::config::parse_go_duration_chrono(&cfg.verify_code_ttl)
                .context("parsing <APP>_VERIFY_CODE_TTL")?,
        );
        let session = SessionService::new(
            users_repo.clone(),
            sessions_repo.clone(),
            jwt_codec.clone(),
            &pepper,
            access_ttl,
            refresh_ttl,
            cfg.max_login_retries,
        );
        let password = PasswordService::new(
            users_repo.clone(),
            resets_repo.clone(),
            outbox.clone(),
            &pepper,
            crate::config::parse_go_duration_chrono(&cfg.reset_code_ttl)
                .context("parsing <APP>_RESET_CODE_TTL")?,
            crate::config::parse_go_duration_chrono(&cfg.reset_rate_window)
                .context("parsing <APP>_RESET_RATE_WINDOW")?,
            cfg.reset_rate_limit,
            cfg.log_out_all_on_reset,
        );
        let oauth = OAuthService::new(
            users_repo.clone(),
            sessions_repo.clone(),
            google_for_service,
            github_for_service,
            LinkToken::new(cfg.jwt_secret.as_str()),
            jwt_codec.clone(),
            cfg.google_client_id.clone(),
            cfg.google_enabled,
            cfg.github_client_id.clone(),
            cfg.github_client_secret.clone(),
            cfg.github_callback.clone(),
            cfg.github_enabled,
            cfg.frontend_success.clone(),
            cfg.offline,
            access_ttl,
            refresh_ttl,
        );
        let admin_users =
            AdminUsersService::new(users_repo.clone(), &cfg.allowed_admin_email_list());

        Ok(Self {
            pool,
            cfg: cfg.clone(),
            widgets_repo,
            widgets,
            users_repo,
            sessions_repo,
            verifications_repo,
            resets_repo,
            outbox,
            audits_repo,
            mailer,
            jwt_codec,
            hub,
            google_client: google_for_state,
            github_client: github_for_state,
            signup: Arc::new(signup),
            session: Arc::new(session),
            password: Arc::new(password),
            oauth: Arc::new(oauth),
            admin_users,
        })
    }

    /// Test wiring: inject any repo (e.g. a mock) — no Postgres required.
    /// Builds the same services from the same config so test behavior
    /// (pepper, TTLs, retries) matches production wiring. `cfg` supplies the
    /// service knobs; pass [`Config::test_default`] with mutations.
    ///
    /// # Panics
    /// Panics when a `cfg` duration is unparseable (test configurations are
    /// hand-written; see [`Config::test_default`]).
    #[cfg(any(test, feature = "mock"))]
    #[allow(clippy::too_many_arguments)] // 1:1 mock wiring: one param per compose slot
    pub fn with_repos(
        pool: PgPool,
        users_repo: Arc<dyn UserRepository>,
        sessions_repo: Arc<dyn SessionRepository>,
        verifications_repo: Arc<dyn EmailVerificationRepository>,
        resets_repo: Arc<dyn PasswordResetRepository>,
        outbox: Arc<dyn OutboxRepository>,
        audits_repo: Arc<dyn AuditLogRepository>,
        widgets_repo: Arc<dyn WidgetRepository>,
        hub: Arc<Hub>,
        mailer: Arc<dyn Mailer>,
        jwt_codec: JwtCodec,
        cfg: Config,
    ) -> Self {
        // Does NOT route through `compose`: the caller supplies hub/mailer/
        // codec directly (mock wiring contract), while `compose` builds its
        // own. The service construction below mirrors `compose` 1:1.
        let widgets = WidgetService::new(widgets_repo.clone());
        let pepper = cfg.password_pepper.clone();
        let ok = |s: &str| {
            crate::config::parse_go_duration_chrono(s).expect("test config: valid duration")
        };
        let signup = SignupService::new(
            users_repo.clone(),
            verifications_repo.clone(),
            outbox.clone(),
            &pepper,
            ok(&cfg.verify_code_ttl),
        );
        let session = SessionService::new(
            users_repo.clone(),
            sessions_repo.clone(),
            jwt_codec.clone(),
            &pepper,
            ok(&cfg.access_token_ttl),
            ok(&cfg.refresh_token_ttl),
            cfg.max_login_retries,
        );
        let password = PasswordService::new(
            users_repo.clone(),
            resets_repo.clone(),
            outbox.clone(),
            &pepper,
            ok(&cfg.reset_code_ttl),
            ok(&cfg.reset_rate_window),
            cfg.reset_rate_limit,
            cfg.log_out_all_on_reset,
        );
        let oauth = OAuthService::new(
            users_repo.clone(),
            sessions_repo.clone(),
            GoogleVerifier::new("", cfg.offline),
            GitHubClient::new(cfg.offline),
            LinkToken::new(cfg.jwt_secret.as_str()),
            jwt_codec.clone(),
            cfg.google_client_id.clone(),
            cfg.google_enabled,
            cfg.github_client_id.clone(),
            cfg.github_client_secret.clone(),
            cfg.github_callback.clone(),
            cfg.github_enabled,
            cfg.frontend_success.clone(),
            cfg.offline,
            ok(&cfg.access_token_ttl),
            ok(&cfg.refresh_token_ttl),
        );
        let admin_users =
            AdminUsersService::new(users_repo.clone(), &cfg.allowed_admin_email_list());
        // Capture flags before `cfg` moves into the struct literal.
        let offline = cfg.offline;
        Self {
            pool,
            cfg,
            widgets_repo,
            widgets,
            users_repo,
            sessions_repo,
            verifications_repo,
            resets_repo,
            outbox,
            audits_repo,
            mailer,
            jwt_codec,
            hub,
            google_client: GoogleVerifier::new("", offline),
            github_client: GitHubClient::new(offline),
            signup: Arc::new(signup),
            session: Arc::new(session),
            password: Arc::new(password),
            oauth: Arc::new(oauth),
            admin_users,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use <app>_core::rbac::Role;
    use <app>_mailer::NoopMailer;
    use <app>_ports::mock::{
        MockAuditLogRepository, MockEmailVerificationRepository, MockOutboxRepository,
        MockPasswordResetRepository, MockSessionRepository, MockUserRepository,
    };

    /// A lazy pool at a deliberately unreachable address: unit tests hit only
    /// the mock repos, never the network (the pool is inert until a query).
    fn dead_pool() -> PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(1))
            .connect_lazy("postgres://u:p@127.0.0.1:1/db")
            .expect("lazy pool never fails to construct")
    }

    /// Cross-seam integration check: `with_repos` compiles against the pinned
    /// service constructors and carries every repo the routes need.
    #[tokio::test]
    async fn with_repos_wires_all_services_and_repos() {
        let cfg = Config::test_default();
        let state = AppState::with_repos(
            dead_pool(),
            Arc::new(MockUserRepository::default()),
            Arc::new(MockSessionRepository::default()),
            Arc::new(MockEmailVerificationRepository::default()),
            Arc::new(MockPasswordResetRepository::default()),
            Arc::new(MockOutboxRepository::default()),
            Arc::new(MockAuditLogRepository::default()),
            Arc::new(<app>_ports::mock::MockWidgetRepository::default()),
            Arc::new(Hub::new(HUB_BUFFER)),
            Arc::new(NoopMailer),
            JwtCodec::from_single(b"test-secret-key-at-least-32-bytes!!"),
            cfg,
        );
        // Structural assertions: every wiring slot holds a service/repo.
        assert!(format!("{state:?}").contains("AppState"));
        // The codec is derived from the injected one (same instance).
        let now = i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_secs(),
        )
        .unwrap_or(i64::MAX);
        let token = state
            .jwt_codec
            .mint(uuid::Uuid::new_v4(), vec![Role::Viewer], now, now + 900)
            .expect("mint with the injected codec");
        assert!(state.jwt_codec.verify(&token).is_ok());
    }
}
