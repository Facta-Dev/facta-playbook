"""Environment-backed configuration, validated once at boot."""

from __future__ import annotations

from pydantic import Field, model_validator
from pydantic_settings import BaseSettings, SettingsConfigDict

_WEAK_SECRETS = {"secret", "changeme", "change-me", "password", "development"}


class Settings(BaseSettings):
    """Every knob the process reads. Unknown variables are ignored."""

    model_config = SettingsConfigDict(
        env_prefix="<APP>_",
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    database_url: str = "postgresql+asyncpg://postgres:postgres@localhost:5432/<app>"
    jwt_secret: str = Field(min_length=32)
    jwt_issuer: str = "<app>"
    access_ttl_minutes: int = Field(default=15, ge=1)
    refresh_ttl_hours: int = Field(default=720, ge=1)
    password_pepper: str = ""

    offline: bool = False
    cors_origins: list[str] = Field(default_factory=list)

    smtp_host: str = ""
    smtp_port: int = 587
    smtp_user: str = ""
    smtp_password: str = ""
    mail_from: str = "no-reply@example.com"

    outbox_poll_seconds: float = Field(default=2.0, gt=0)
    outbox_batch: int = Field(default=10, ge=1)
    janitor_poll_seconds: int = Field(default=3600, ge=1)

    verify_code_ttl_hours: int = Field(default=24, ge=1)
    verify_max_attempts: int = Field(default=5, ge=1)
    reset_ttl_minutes: int = Field(default=30, ge=1)
    reset_rate_limit: int = Field(default=3, ge=1)

    google_client_id: str = ""
    github_client_id: str = ""
    github_client_secret: str = ""
    github_callback_url: str = "http://localhost:8000/auth/github/callback"
    frontend_success: str = "http://localhost:3000"
    allowed_admin_emails: list[str] = Field(default_factory=list)

    log_level: str = "INFO"

    @model_validator(mode="after")
    def _fail_closed(self) -> Settings:
        if not self.database_url.startswith("postgresql+asyncpg://"):
            msg = "database_url must use the postgresql+asyncpg driver"
            raise ValueError(msg)
        if self.jwt_secret.lower() in _WEAK_SECRETS or len(set(self.jwt_secret)) < 8:
            msg = "jwt_secret is too weak; use at least 32 random characters"
            raise ValueError(msg)
        if self.offline and (self.smtp_password or self.smtp_user):
            msg = "offline mode forbids SMTP credentials"
            raise ValueError(msg)
        return self
