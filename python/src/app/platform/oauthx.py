"""Google and GitHub identity verification behind one narrow surface."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

import anyio
import httpx
import jwt
from jwt import PyJWKClient

from app.platform.config import Settings
from app.shared.apperr import Code, E

_GOOGLE_CERTS = "https://www.googleapis.com/oauth2/v3/certs"
_GITHUB_ACCESS_URL = "https://github.com/login/oauth/access_token"
_GITHUB_USER = "https://api.github.com/user"
_GITHUB_EMAILS = "https://api.github.com/user/emails"
_TIMEOUT = httpx.Timeout(10.0, connect=5.0)


@dataclass(frozen=True)
class ExternalIdentity:
    """The provider facts the application trusts."""

    provider: str
    subject: str
    email: str
    name: str


def _require_online(settings: Settings) -> None:
    if settings.offline:
        msg = "outbound identity calls are disabled while offline"
        raise E(Code.AUTH_OFFLINE, msg, 503)


class GoogleVerifier:
    """Verifies Google ID tokens against the cached JWKS."""

    def __init__(self, settings: Settings) -> None:
        self._settings = settings
        self._keys = PyJWKClient(_GOOGLE_CERTS, cache_keys=True)

    @property
    def configured(self) -> bool:
        return bool(self._settings.google_client_id)

    async def verify(self, id_token: str) -> ExternalIdentity:
        _require_online(self._settings)
        if not self.configured:
            msg = "google_client_id is not configured"
            raise E(Code.AUTH_OAUTH_FAILED, msg, 503)
        try:
            signing_key = await anyio.to_thread.run_sync(
                self._keys.get_signing_key_from_jwt, id_token
            )
            claims = jwt.decode(
                id_token,
                signing_key.key,
                algorithms=["RS256"],
                audience=self._settings.google_client_id,
                options={"require": ["sub", "email", "aud", "exp"]},
            )
        except (jwt.PyJWTError, httpx.HTTPError) as exc:
            msg = "google identity token rejected"
            raise E(Code.AUTH_OAUTH_FAILED, msg, 401) from exc
        return ExternalIdentity(
            provider="google",
            subject=str(claims["sub"]),
            email=str(claims["email"]),
            name=str(claims.get("name", "")),
        )


class GithubOAuth:
    """Authorization-code exchange and profile lookup."""

    def __init__(self, settings: Settings) -> None:
        self._settings = settings

    @property
    def configured(self) -> bool:
        return bool(self._settings.github_client_id and self._settings.github_client_secret)

    def authorize_url(self, redirect_uri: str, state: str) -> str:
        params = httpx.QueryParams(
            client_id=self._settings.github_client_id,
            redirect_uri=redirect_uri,
            scope="read:user user:email",
            state=state,
        )
        return f"https://github.com/login/oauth/authorize?{params}"

    async def exchange(self, code: str) -> ExternalIdentity:
        _require_online(self._settings)
        if not self.configured:
            msg = "github oauth is not configured"
            raise E(Code.AUTH_OAUTH_FAILED, msg, 503)
        try:
            async with httpx.AsyncClient(timeout=_TIMEOUT) as client:
                token_response = await client.post(
                    _GITHUB_ACCESS_URL,
                    data={
                        "client_id": self._settings.github_client_id,
                        "client_secret": self._settings.github_client_secret,
                        "code": code,
                    },
                    headers={"accept": "application/json"},
                )
                token_response.raise_for_status()
                token = str(token_response.json().get("access_token", ""))
                if not token:
                    msg = "github returned no access token"
                    raise E(Code.AUTH_OAUTH_FAILED, msg, 401)
                headers = {"authorization": f"Bearer {token}"}
                user_response = await client.get(_GITHUB_USER, headers=headers)
                user_response.raise_for_status()
                profile: dict[str, Any] = user_response.json()
                email = profile.get("email")
                if not email:
                    emails_response = await client.get(_GITHUB_EMAILS, headers=headers)
                    emails_response.raise_for_status()
                    for candidate in emails_response.json():
                        if candidate.get("primary") and candidate.get("verified"):
                            email = candidate.get("email")
                            break
        except httpx.HTTPError as exc:
            msg = "github identity exchange failed"
            raise E(Code.AUTH_OAUTH_FAILED, msg, 401) from exc
        if not email:
            msg = "github account has no verified email"
            raise E(Code.AUTH_OAUTH_FAILED, msg, 401)
        return ExternalIdentity(
            provider="github",
            subject=str(profile["id"]),
            email=str(email),
            name=str(profile.get("name") or profile.get("login") or ""),
        )
