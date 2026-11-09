"""Stable error codes and the one application exception type."""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum


class Code(StrEnum):
    """Stable machine-readable codes; clients match on these, never on text."""

    VALIDATION_FAILED = "VALIDATION_FAILED"
    UNAUTHENTICATED = "UNAUTHENTICATED"
    FORBIDDEN = "FORBIDDEN"
    NOT_FOUND = "NOT_FOUND"
    CONFLICT = "CONFLICT"
    RATE_LIMITED = "RATE_LIMITED"
    INTERNAL = "INTERNAL"

    AUTH_EMAIL_TAKEN = "AUTH_EMAIL_TAKEN"
    AUTH_INVALID_CREDENTIALS = "AUTH_INVALID_CREDENTIALS"
    AUTH_ACCOUNT_BLOCKED = "AUTH_ACCOUNT_BLOCKED"
    AUTH_ACCOUNT_INACTIVE = "AUTH_ACCOUNT_INACTIVE"
    AUTH_EMAIL_UNVERIFIED = "AUTH_EMAIL_UNVERIFIED"
    AUTH_INVALID_TOKEN = "AUTH_INVALID_TOKEN"
    AUTH_CODE_INVALID = "AUTH_CODE_INVALID"
    AUTH_CODE_EXPIRED = "AUTH_CODE_EXPIRED"
    AUTH_CODE_ATTEMPTS = "AUTH_CODE_ATTEMPTS"
    AUTH_OFFLINE = "AUTH_OFFLINE"
    AUTH_OAUTH_FAILED = "AUTH_OAUTH_FAILED"

    WIDGET_NOT_FOUND = "WIDGET_NOT_FOUND"


@dataclass(eq=False)
class AppError(Exception):
    """A typed failure that crosses every layer unchanged."""

    code: Code
    detail: str
    status: int = 400

    def __post_init__(self) -> None:
        super().__init__(self.detail)

    def __str__(self) -> str:
        return f"{self.code.value}: {self.detail}"


def E(code: Code, detail: str, status: int = 400) -> AppError:
    """Build an ``AppError`` with an explicit status."""
    return AppError(code=code, detail=detail, status=status)
