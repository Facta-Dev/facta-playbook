"""Signup and email-verification types. Pure data; no I/O imports."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime

from app.shared.rolex import Role


@dataclass(frozen=True)
class SignupUser:
    """The row a signup creates before verification."""

    id: str
    email: str
    password_hash: str
    role: Role
    first_name: str
    last_name: str
    language: str
    is_verified: bool
    created_at: datetime


@dataclass(frozen=True)
class VerificationCode:
    """A six-digit code with its attempt budget."""

    id: str
    user_id: str
    code: str
    expires_at: datetime
    attempts: int


@dataclass(frozen=True)
class SignupResult:
    """What the route returns on success."""

    user_id: str
    code_queued: bool
