"""JWT access tokens, password hashing, and the auth dependencies."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import timedelta
from typing import Annotated, Any

import jwt
from argon2 import PasswordHasher
from argon2.exceptions import InvalidHashError, VerificationError, VerifyMismatchError
from fastapi import Depends, Request
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer

from app.platform.config import Settings
from app.shared.apperr import Code, E
from app.shared.clockx import Clock
from app.shared.rolex import Role, at_least

_bearer = HTTPBearer(auto_error=False)
_hasher = PasswordHasher()


class Passwords:
    """Argon2id hashing; the pepper is optional and config-supplied."""

    def __init__(self, pepper: str = "") -> None:
        self._pepper = pepper

    def hash(self, plain: str) -> str:
        return _hasher.hash(plain + self._pepper)

    def verify(self, stored: str, plain: str) -> bool:
        try:
            return _hasher.verify(stored, plain + self._pepper)
        except (VerifyMismatchError, VerificationError, InvalidHashError):
            return False

    def needs_rehash(self, stored: str) -> bool:
        try:
            return _hasher.check_needs_rehash(stored)
        except InvalidHashError:
            return True


@dataclass(frozen=True)
class Identity:
    """The authenticated caller, carried by every protected route."""

    user_id: str
    email: str
    role: Role


class Tokens:
    """Mints and verifies HS256 access tokens and short-lived state JWTs."""

    _ALGORITHM = "HS256"

    def __init__(self, settings: Settings, clock: Clock) -> None:
        self._settings = settings
        self._clock = clock

    def mint_access(self, identity: Identity) -> str:
        now = self._clock.now()
        claims: dict[str, Any] = {
            "sub": identity.user_id,
            "email": identity.email,
            "role": identity.role.value,
            "iss": self._settings.jwt_issuer,
            "iat": now,
            "exp": now + timedelta(minutes=self._settings.access_ttl_minutes),
        }
        return jwt.encode(claims, self._settings.jwt_secret, algorithm=self._ALGORITHM)

    def verify_access(self, token: str) -> Identity:
        try:
            claims = jwt.decode(
                token,
                self._settings.jwt_secret,
                algorithms=[self._ALGORITHM],
                issuer=self._settings.jwt_issuer,
                options={"require": ["sub", "exp", "iss", "role"]},
            )
        except jwt.PyJWTError as exc:
            msg = "invalid or expired access token"
            raise E(Code.AUTH_INVALID_TOKEN, msg, 401) from exc
        role = Role(claims["role"])
        return Identity(user_id=str(claims["sub"]), email=str(claims["email"]), role=role)

    def mint_state(self, claims: dict[str, Any], ttl_seconds: int = 600) -> str:
        now = self._clock.now()
        payload = {**claims, "iss": self._settings.jwt_issuer, "iat": now}
        payload["exp"] = now + timedelta(seconds=ttl_seconds)
        return jwt.encode(payload, self._settings.jwt_secret, algorithm=self._ALGORITHM)

    def verify_state(self, token: str) -> dict[str, Any]:
        try:
            decoded = jwt.decode(
                token,
                self._settings.jwt_secret,
                algorithms=[self._ALGORITHM],
                issuer=self._settings.jwt_issuer,
                options={"require": ["exp", "iss"]},
            )
        except jwt.PyJWTError as exc:
            msg = "invalid or expired state token"
            raise E(Code.AUTH_INVALID_TOKEN, msg, 400) from exc
        return {str(key): value for key, value in decoded.items()}


async def require_auth(
    request: Request,
    credentials: Annotated[HTTPAuthorizationCredentials | None, Depends(_bearer)],
) -> Identity:
    """Dependency: verify the bearer token, or answer 401."""
    if credentials is None:
        msg = "missing bearer token"
        raise E(Code.UNAUTHENTICATED, msg, 401)
    tokens: Tokens = request.app.state.tokens
    return tokens.verify_access(credentials.credentials)


def require_role(floor: Role) -> Any:
    """Dependency factory: require at least ``floor``."""

    async def dependency(identity: Annotated[Identity, Depends(require_auth)]) -> Identity:
        if not at_least(identity.role, floor):
            msg = f"requires role {floor.value} or higher"
            raise E(Code.FORBIDDEN, msg, 403)
        return identity

    return dependency


IdentityDep = Annotated[Identity, Depends(require_auth)]
