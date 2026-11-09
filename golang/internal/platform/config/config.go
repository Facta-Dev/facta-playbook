// Package config loads typed configuration from the environment (env/v11)
// and validates secret strength. The <APP>_ env prefix is substituted at
// instantiation time by scripts/rename.sh.
package config

import (
	"fmt"
	"strings"
	"time"

	"github.com/caarlos0/env/v11"
)

type Config struct {
	// Runtime
	Env      string `env:"<APP>_ENV" envDefault:"development"`
	Bind     string `env:"<APP>_BIND" envDefault:"0.0.0.0:8080"`
	LogLevel string `env:"<APP>_LOG_LEVEL" envDefault:"info"`

	// CORS allowlist; comma-separated exact origins. "*" matches everything
	// (development only) — the check is exact otherwise.
	CORSOrigins []string `env:"<APP>_CORS_ORIGINS" envDefault:"http://localhost:5173"`

	// Postgres
	DatabaseURL string `env:"<APP>_DATABASE_URL,required"`
	DBMaxConns  int32  `env:"<APP>_DATABASE_MAX_CONNECTIONS" envDefault:"20"`

	// Auth / security
	JWTSecret        string        `env:"<APP>_JWT_SECRET"`      // required at boot (validated below)
	Pepper           string        `env:"<APP>_PASSWORD_PEPPER"` // optional; empty = disabled
	AccessTokenTTL   time.Duration `env:"<APP>_ACCESS_TOKEN_TTL" envDefault:"15m"`
	RefreshTokenTTL  time.Duration `env:"<APP>_REFRESH_TOKEN_TTL" envDefault:"720h"`
	VerifyCodeTTL    time.Duration `env:"<APP>_VERIFY_CODE_TTL" envDefault:"24h"`
	ResetCodeTTL     time.Duration `env:"<APP>_RESET_CODE_TTL" envDefault:"30m"`
	AuthCookie       bool          `env:"<APP>_AUTH_COOKIE" envDefault:"false"` // also set cookie "<app>_access_token"
	ResetRateLimit   int           `env:"<APP>_RESET_RATE_LIMIT" envDefault:"3"`
	ResetRateWindow  time.Duration `env:"<APP>_RESET_RATE_WINDOW" envDefault:"5m"`
	MaxLoginRetries  int           `env:"<APP>_MAX_LOGIN_RETRIES" envDefault:"5"`
	LogOutAllOnReset bool          `env:"<APP>_LOGOUT_ALL_ON_RESET" envDefault:"true"`

	// OAuth sign-in (reference parity). Google verifies the browser's One Tap ID
	// token (empty client id = unconfigured, endpoint answers 401). GitHub
	// ships disabled (reference default): both github routes answer 403 until
	// enabled with id + secret. Both flows perform outbound sockets, so they
	// refuse with a config-style error while <APP>_OFFLINE=true (INV-T7).
	GoogleClientID  string `env:"<APP>_GOOGLE_CLIENT_ID"`
	GoogleEnabled   bool   `env:"<APP>_GOOGLE_ENABLED" envDefault:"true"` // hard-off switch when an id exists
	GitHubClientID  string `env:"<APP>_GITHUB_CLIENT_ID"`
	GitHubSecret    string `env:"<APP>_GITHUB_CLIENT_SECRET"`
	GitHubEnabled   bool   `env:"<APP>_GITHUB_ENABLED" envDefault:"false"`
	GitHubCallback  string `env:"<APP>_AUTH_GITHUB_CALLBACK_URL" envDefault:"http://localhost:8080/api/v1/auth/github/callback"`
	FrontendSuccess string `env:"<APP>_AUTH_FRONTEND_SUCCESS" envDefault:"http://localhost:5173/auth/success"`

	// Janitor: worker-loop purge of expired sessions and stale codes (1h).
	JanitorPoll time.Duration `env:"<APP>_JANITOR_POLL" envDefault:"1h"`

	// Bootstrap whitelist for the unauthenticated POST /auth/upgrade-to-admin
	// route (reference upgrade_to_admin parity). Comma-separated emails,
	// matched case-insensitively; empty disables the route entirely.
	AllowedAdminEmails []string `env:"<APP>_ALLOWED_ADMIN_EMAILS" envSeparator:","`

	// SMTP (empty => console sender; <APP>_OFFLINE=true forces console, INV-T7)
	SMTPHost string `env:"<APP>_SMTP_HOST"`
	SMTPPort int    `env:"<APP>_SMTP_PORT" envDefault:"587"`
	SMTPUser string `env:"<APP>_SMTP_USER"`
	SMTPPass string `env:"<APP>_SMTP_PASS"`
	SMTPFrom string `env:"<APP>_SMTP_FROM"`

	// INV-T7 offline guard: true => console mailer, no sockets opened.
	Offline bool `env:"<APP>_OFFLINE" envDefault:"false"`

	// Email outbox worker
	OutboxPoll time.Duration `env:"<APP>_OUTBOX_POLL" envDefault:"2s"`
	OutboxSize int           `env:"<APP>_OUTBOX_BATCH" envDefault:"10"`
}

// Load parses the environment and enforces the secret rules.
func Load() (Config, error) {
	cfg := Config{}
	if err := env.Parse(&cfg); err != nil {
		return Config{}, fmt.Errorf("config: %w", err)
	}
	if len(cfg.JWTSecret) < 32 ||
		strings.Contains(cfg.JWTSecret, "<set-me") ||
		strings.Contains(cfg.JWTSecret, "change-me-32-bytes") {
		return Config{}, fmt.Errorf("config: <APP>_JWT_SECRET must be 32+ random bytes (openssl rand -base64 32); placeholder/default rejected")
	}
	if cfg.Pepper != "" && len(cfg.Pepper) < 32 {
		return Config{}, fmt.Errorf("config: <APP>_PASSWORD_PEPPER, when set, must be 32+ bytes")
	}
	if cfg.Offline && (cfg.SMTPHost != "" || cfg.SMTPUser != "" || cfg.SMTPPass != "") {
		return Config{}, fmt.Errorf("config: <APP>_OFFLINE=true forbids SMTP credentials (INV-T7); use the console sender")
	}
	return cfg, nil
}
