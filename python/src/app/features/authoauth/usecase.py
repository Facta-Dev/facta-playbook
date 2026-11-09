"""Google One-Tap and GitHub OAuth on top of the session plumbing."""

from __future__ import annotations

from datetime import timedelta

from app.features.authoauth.domain import (
    LINK_PURPOSE,
    OAuthSession,
    OAuthTokens,
    OAuthUser,
)
from app.features.authoauth.ports import GithubClient, GoogleVerifier, OAuthStore
from app.platform.authx import Identity, Tokens
from app.platform.oauthx import ExternalIdentity
from app.shared.apperr import AppError, Code, E
from app.shared.clockx import Clock
from app.shared.idgen import IdSource
from app.shared.rolex import Role


class OAuthService:
    """Provider identity resolves to an account, then a session."""

    def __init__(
        self,
        store: OAuthStore,
        google: GoogleVerifier,
        github: GithubClient,
        tokens: Tokens,
        ids: IdSource,
        clock: Clock,
        callback_url: str,
        frontend_success: str,
        offline: bool,
        refresh_ttl_hours: int = 720,
    ) -> None:
        self._store = store
        self._google = google
        self._github = github
        self._tokens = tokens
        self._ids = ids
        self._clock = clock
        self._callback_url = callback_url
        self._frontend = frontend_success
        self._offline = offline
        self._refresh_ttl = timedelta(hours=refresh_ttl_hours)

    async def _session(self, user: OAuthUser, ip: str, user_agent: str) -> OAuthTokens:
        self._gate(user)
        now = self._clock.now()
        access = self._tokens.mint_access(
            Identity(user_id=user.id, email=user.email, role=user.role)
        )
        refresh = self._ids.opaque_token()
        await self._store.create_session(
            OAuthSession(
                id=self._ids.new_id(),
                user_id=user.id,
                refresh_token=refresh,
                ip=ip,
                user_agent=user_agent,
                created_at=now,
                expires_at=now + self._refresh_ttl,
            )
        )
        return OAuthTokens(access_token=access, refresh_token=refresh)

    @staticmethod
    def _gate(user: OAuthUser) -> None:
        if user.is_blocked:
            msg = "account is blocked"
            raise E(Code.AUTH_ACCOUNT_BLOCKED, msg, 403)
        if not user.is_active:
            msg = "account is inactive"
            raise E(Code.AUTH_ACCOUNT_INACTIVE, msg, 403)

    async def _resolve(
        self,
        found: OAuthUser | None,
        by_email: OAuthUser | None,
        external: ExternalIdentity,
        provider: str,
    ) -> OAuthUser:
        if found is not None:
            return found
        if by_email is not None:
            if (by_email.google_id if provider == "google" else by_email.github_id) != "":
                msg = f"{provider} account is already linked to another user"
                raise E(Code.AUTH_EMAIL_TAKEN, msg, 409)
            if provider == "google":
                await self._store.link_google(by_email.id, external.subject, "")
            else:
                await self._store.link_github(by_email.id, external.subject, "")
            return by_email
        provisioned = OAuthUser(
            id=self._ids.new_id(),
            email=external.email.strip().lower(),
            role=Role.USER,
            first_name=external.name,
            last_name="",
            language="en",
            is_active=True,
            is_blocked=False,
            is_verified=True,
            google_id=external.subject if provider == "google" else "",
            github_id=external.subject if provider == "github" else "",
        )
        await self._store.create_user(provisioned, "", "", self._clock.now())
        return provisioned

    async def google_login(self, credential: str, ip: str, user_agent: str) -> OAuthTokens:
        """Verify the Google credential and issue tokens."""
        external = await self._google.verify(credential)
        user = await self._resolve(
            await self._store.find_by_google(external.subject),
            await self._store.find_by_email(external.email.strip().lower()),
            external,
            "google",
        )
        return await self._session(user, ip, user_agent)

    async def github_login_url(self, state: str) -> str:
        """Consent URL; unconfigured or offline is an operator error."""
        if self._offline or not self._github.configured:
            msg = "github login is not configured"
            raise E(Code.AUTH_OAUTH_FAILED, msg, 503)
        return self._github.authorize_url(self._callback_url, state)

    async def github_login(self, code: str, ip: str, user_agent: str) -> OAuthTokens:
        """Exchange the code and issue tokens."""
        external = await self._github.exchange(code)
        user = await self._resolve(
            await self._store.find_by_github(external.subject),
            await self._store.find_by_email(external.email.strip().lower()),
            external,
            "github",
        )
        return await self._session(user, ip, user_agent)

    async def github_connect_url(self, user_id: str) -> str:
        """Consent URL whose state carries the caller's link intent."""
        if self._offline or not self._github.configured:
            msg = "github login is not configured"
            raise E(Code.AUTH_OAUTH_FAILED, msg, 503)
        state = self._tokens.mint_state({"purpose": LINK_PURPOSE, "sub": user_id})
        return self._github.authorize_url(self._callback_url, state)

    async def github_link(self, state: str, code: str) -> None:
        """Attach a GitHub subject to the account named in the state."""
        claims = self._tokens.verify_state(state)
        if claims.get("purpose") != LINK_PURPOSE or not claims.get("sub"):
            msg = "this state was not issued for linking"
            raise E(Code.AUTH_INVALID_TOKEN, msg, 400)
        user_id = str(claims["sub"])
        external = await self._github.exchange(code)
        owner = await self._store.find_by_github(external.subject)
        if owner is not None and owner.id != user_id:
            msg = "github account is already linked to another user"
            raise E(Code.AUTH_EMAIL_TAKEN, msg, 409)
        await self._store.link_github(user_id, external.subject, "")

    def is_link_state(self, state: str) -> bool:
        """True when the state is a link token; a bad token is a login flow."""
        try:
            claims = self._tokens.verify_state(state)
        except AppError:
            return False
        return claims.get("purpose") == LINK_PURPOSE

    @property
    def frontend_success(self) -> str:
        """Where the browser lands after a GitHub flow."""
        return self._frontend
