"""In-memory adapters for the unit and acceptance tiers.

Every multi-write method stages both writes before returning, mirroring the
real adapter's single transaction. Unit ids and codes are deterministic so a
test can assert on them.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import UTC, datetime
from typing import Any
from urllib.parse import urlencode

from app.features.adminusers.domain import AdminUser, Patch
from app.features.authoauth.domain import OAuthSession, OAuthUser
from app.features.authpassword.domain import PasswordAccount, ResetCode
from app.features.authsession.domain import Account, Session, SessionAnomaly
from app.features.authsignup.domain import SignupUser, VerificationCode
from app.features.widgets.domain import Widget, WidgetKind
from app.platform.authx import Identity
from app.platform.email.outbox import EmailMessage
from app.platform.oauthx import ExternalIdentity
from app.shared.apperr import Code, E
from app.shared.rolex import Role


@dataclass
class World:
    """The shared state every fake adapter reads and writes."""

    users: dict[str, dict[str, Any]] = field(default_factory=dict)
    codes: list[dict[str, Any]] = field(default_factory=list)
    resets: list[dict[str, Any]] = field(default_factory=list)
    sessions: dict[str, dict[str, Any]] = field(default_factory=dict)
    jobs: list[dict[str, Any]] = field(default_factory=list)
    widgets: dict[str, dict[str, Any]] = field(default_factory=dict)
    audits: list[dict[str, Any]] = field(default_factory=list)
    seq: int = 0

    def next_id(self) -> str:
        self.seq += 1
        return f"00000000-0000-0000-0000-{self.seq:012d}"

    def latest_code(self, email: str) -> str:
        user = self.user_by_email(email)
        assert user is not None
        code = next(c for c in reversed(self.codes) if c["user_id"] == user["id"])
        return str(code["code"])

    def latest_job_code(self) -> str:
        assert self.jobs
        return str(self.jobs[-1]["variables"]["code"])

    def user_by_email(self, email: str) -> dict[str, Any] | None:
        for user in self.users.values():
            if user["email"] == email.strip().lower():
                return user
        return None

    def emails_for(self, template_key: str, to: str) -> list[dict[str, Any]]:
        """The queued email jobs for one template and one address."""
        wanted = to.strip().lower()
        return [
            job for job in self.jobs if job["template_key"] == template_key and job["to"] == wanted
        ]

    def count_emails(self, template_key: str, to: str) -> int:
        """How many emails of one template went to one address."""
        return len(self.emails_for(template_key, to))

    def latest_email_code(self, template_key: str, to: str) -> str:
        """The newest code emailed for one template and one address."""
        jobs = self.emails_for(template_key, to)
        assert jobs, f"no {template_key} email was enqueued for {to}"
        return str(jobs[-1]["variables"]["code"])

    def add_user(
        self,
        email: str,
        password_hash: str = "hash",
        role: Role = Role.USER,
        verified: bool = True,
        active: bool = True,
        blocked: bool = False,
    ) -> dict[str, Any]:
        user = {
            "id": self.next_id(),
            "email": email.strip().lower(),
            "password_hash": password_hash,
            "role": role,
            "first_name": "",
            "last_name": "",
            "language": "en",
            "is_active": active,
            "is_blocked": blocked,
            "is_verified": verified,
            "last_login": None,
            "google_id": "",
            "github_id": "",
            "created_at": datetime(2026, 1, 1, tzinfo=UTC),
        }
        self.users[user["id"]] = user
        return user


class SequenceIds:
    """Deterministic id and code source for tests."""

    def __init__(self, world: World) -> None:
        self._world = world
        self._tokens = 0

    def new_id(self) -> str:
        return self._world.next_id()

    def opaque_token(self) -> str:
        self._tokens += 1
        return f"token-{self._tokens:04d}".ljust(48, "0")

    def numeric_code(self) -> str:
        return "123456"


class FakeSignupStore:
    def __init__(self, world: World) -> None:
        self.w = world

    def _user(self, row: dict[str, Any]) -> SignupUser:
        return SignupUser(
            id=row["id"],
            email=row["email"],
            password_hash=row["password_hash"],
            role=row["role"],
            first_name=row["first_name"],
            last_name=row["last_name"],
            language=row["language"],
            is_verified=row["is_verified"],
            created_at=row["created_at"],
        )

    async def find_by_email(self, email: str) -> SignupUser | None:
        row = self.w.user_by_email(email)
        return None if row is None else self._user(row)

    def _code(self, code: VerificationCode) -> dict[str, Any]:
        return {
            "id": code.id,
            "user_id": code.user_id,
            "code": code.code,
            "expires_at": code.expires_at,
            "attempts": code.attempts,
        }

    async def register(
        self, user: SignupUser, code: VerificationCode, message: EmailMessage, now: datetime
    ) -> None:
        self.w.users[user.id] = {
            "id": user.id,
            "email": user.email,
            "password_hash": user.password_hash,
            "role": user.role,
            "first_name": user.first_name,
            "last_name": user.last_name,
            "language": user.language,
            "is_active": True,
            "is_blocked": False,
            "is_verified": False,
            "last_login": None,
            "google_id": "",
            "github_id": "",
            "created_at": user.created_at,
        }
        self.w.codes.append(self._code(code))
        self.w.jobs.append(self._job(message))

    async def replace_code(
        self, user_id: str, code: VerificationCode, message: EmailMessage, now: datetime
    ) -> None:
        self.w.codes = [c for c in self.w.codes if c["user_id"] != user_id]
        self.w.codes.append(self._code(code))
        self.w.jobs.append(self._job(message))

    @staticmethod
    def _job(message: EmailMessage) -> dict[str, Any]:
        return {
            "to": message.to,
            "template_key": message.template_key,
            "variables": dict(message.variables),
        }

    async def find_code(self, user_id: str) -> VerificationCode | None:
        for row in reversed(self.w.codes):
            if row["user_id"] == user_id:
                return VerificationCode(**row)
        return None

    async def increment_attempts(self, code_id: str) -> None:
        for row in self.w.codes:
            if row["id"] == code_id:
                row["attempts"] += 1

    async def consume_code(self, code_id: str, user_id: str) -> None:
        self.w.codes = [c for c in self.w.codes if c["id"] != code_id]
        self.w.users[user_id]["is_verified"] = True


class FakeSessionStore:
    def __init__(self, world: World) -> None:
        self.w = world

    def _account(self, row: dict[str, Any]) -> Account:
        return Account(
            id=row["id"],
            email=row["email"],
            password_hash=row["password_hash"],
            role=row["role"],
            first_name=row["first_name"],
            last_name=row["last_name"],
            language=row["language"],
            is_active=row["is_active"],
            is_blocked=row["is_blocked"],
            is_verified=row["is_verified"],
            last_login=row["last_login"],
        )

    async def find_account(self, email: str) -> Account | None:
        row = self.w.user_by_email(email)
        return None if row is None else self._account(row)

    async def get_account(self, user_id: str) -> Account | None:
        row = self.w.users.get(user_id)
        return None if row is None else self._account(row)

    async def record_login(self, user_id: str, now: datetime) -> None:
        self.w.users[user_id]["last_login"] = now

    async def create_session(self, session: Session) -> None:
        self.w.sessions[session.refresh_token] = {
            "id": session.id,
            "user_id": session.user_id,
            "refresh_token": session.refresh_token,
            "ip": session.ip,
            "user_agent": session.user_agent,
            "is_active": True,
            "created_at": session.created_at,
            "expires_at": session.expires_at,
        }

    async def find_session(self, token: str) -> Session | None:
        row = self.w.sessions.get(token)
        return None if row is None else Session(**row)

    async def rotate(
        self,
        session_id: str,
        old_token: str,
        new_token: str,
        expires_at: datetime,
        now: datetime,
    ) -> bool:
        for token, row in list(self.w.sessions.items()):
            if row["id"] != session_id or row["is_active"] is False:
                continue
            if token != old_token or row["expires_at"] <= now:
                return False
            del self.w.sessions[token]
            row["refresh_token"] = new_token
            row["expires_at"] = expires_at
            self.w.sessions[new_token] = row
            return True
        return False

    async def deactivate(self, token: str) -> bool:
        row = self.w.sessions.get(token)
        if row is None or not row["is_active"]:
            return False
        row["is_active"] = False
        return True

    async def deactivate_all(self, user_id: str) -> int:
        count = 0
        for row in self.w.sessions.values():
            if row["user_id"] == user_id and row["is_active"]:
                row["is_active"] = False
                count += 1
        return count

    async def list_sessions(self, user_id: str, now: datetime) -> list[Session]:
        return [
            Session(**row)
            for row in self.w.sessions.values()
            if row["user_id"] == user_id and row["is_active"] and row["expires_at"] > now
        ]

    async def update_profile(
        self, user_id: str, first_name: str, last_name: str, language: str
    ) -> Account | None:
        row = self.w.users.get(user_id)
        if row is None:
            return None
        row.update(first_name=first_name, last_name=last_name, language=language)
        return self._account(row)

    async def anomalies(self, now: datetime) -> list[SessionAnomaly]:
        grouped: dict[str, set[str]] = {}
        counts: dict[str, int] = {}
        for row in self.w.sessions.values():
            if not row["is_active"] or row["expires_at"] <= now:
                continue
            grouped.setdefault(row["user_id"], set()).add(row["ip"])
            counts[row["user_id"]] = counts.get(row["user_id"], 0) + 1
        return [
            SessionAnomaly(
                user_id=user_id,
                email=self.w.users[user_id]["email"],
                distinct_ips=len(ips),
                sessions=counts[user_id],
            )
            for user_id, ips in grouped.items()
            if len(ips) > 1 and user_id in self.w.users
        ]


class FakePasswordStore:
    def __init__(self, world: World) -> None:
        self.w = world

    async def find_account(self, email: str) -> PasswordAccount | None:
        row = self.w.user_by_email(email)
        if row is None:
            return None
        return PasswordAccount(id=row["id"], email=row["email"], password_hash=row["password_hash"])

    async def recent_resets(self, email: str, since: datetime) -> int:
        return sum(1 for r in self.w.resets if r["email"] == email and r["created_at"] >= since)

    async def create_reset(self, reset: ResetCode, message: EmailMessage, now: datetime) -> None:
        self.w.resets.append({**reset.__dict__, "created_at": now})
        self.w.jobs.append(
            {
                "to": message.to,
                "template_key": message.template_key,
                "variables": dict(message.variables),
            }
        )

    async def find_reset(self, email: str) -> ResetCode | None:
        for row in reversed(self.w.resets):
            if row["email"] == email:
                return ResetCode(
                    id=row["id"],
                    email=row["email"],
                    code=row["code"],
                    expires_at=row["expires_at"],
                    attempts=row["attempts"],
                )
        return None

    async def increment_attempts(self, reset_id: str) -> None:
        for row in self.w.resets:
            if row["id"] == reset_id:
                row["attempts"] += 1

    async def consume_reset(self, email: str, user_id: str, password_hash: str) -> None:
        self.w.resets = [r for r in self.w.resets if r["email"] != email]
        self.w.users[user_id]["password_hash"] = password_hash
        for row in self.w.sessions.values():
            if row["user_id"] == user_id:
                row["is_active"] = False

    async def set_password(self, user_id: str, password_hash: str) -> None:
        self.w.users[user_id]["password_hash"] = password_hash

    async def revoke_sessions(self, user_id: str) -> int:
        count = 0
        for row in self.w.sessions.values():
            if row["user_id"] == user_id and row["is_active"]:
                row["is_active"] = False
                count += 1
        return count


class FakeOAuthStore:
    def __init__(self, world: World) -> None:
        self.w = world

    def _user(self, row: dict[str, Any]) -> OAuthUser:
        return OAuthUser(
            id=row["id"],
            email=row["email"],
            role=row["role"],
            first_name=row["first_name"],
            last_name=row["last_name"],
            language=row["language"],
            is_active=row["is_active"],
            is_blocked=row["is_blocked"],
            is_verified=row["is_verified"],
            google_id=row["google_id"],
            github_id=row["github_id"],
        )

    async def find_by_google(self, subject: str) -> OAuthUser | None:
        for row in self.w.users.values():
            if row["google_id"] == subject:
                return self._user(row)
        return None

    async def find_by_github(self, subject: str) -> OAuthUser | None:
        for row in self.w.users.values():
            if row["github_id"] == subject:
                return self._user(row)
        return None

    async def find_by_email(self, email: str) -> OAuthUser | None:
        row = self.w.user_by_email(email)
        return None if row is None else self._user(row)

    async def find_by_id(self, user_id: str) -> OAuthUser | None:
        row = self.w.users.get(user_id)
        return None if row is None else self._user(row)

    async def create_user(
        self, user: OAuthUser, password_hash: str, picture: str, now: Any
    ) -> None:
        self.w.users[user.id] = {
            "id": user.id,
            "email": user.email,
            "password_hash": password_hash,
            "role": user.role,
            "first_name": user.first_name,
            "last_name": user.last_name,
            "language": user.language,
            "is_active": True,
            "is_blocked": False,
            "is_verified": True,
            "last_login": None,
            "google_id": user.google_id,
            "github_id": user.github_id,
            "created_at": now,
        }

    async def link_google(self, user_id: str, subject: str, picture: str) -> None:
        self.w.users[user_id].update(google_id=subject, is_verified=True)

    async def link_github(self, user_id: str, subject: str, picture: str) -> None:
        self.w.users[user_id].update(github_id=subject, is_verified=True)

    async def create_session(self, session: OAuthSession) -> None:
        self.w.sessions[session.refresh_token] = {
            "id": session.id,
            "user_id": session.user_id,
            "refresh_token": session.refresh_token,
            "ip": session.ip,
            "user_agent": session.user_agent,
            "is_active": True,
            "created_at": session.created_at,
            "expires_at": session.expires_at,
        }


class FakeAdminUserStore:
    def __init__(self, world: World) -> None:
        self.w = world

    def _admin(self, row: dict[str, Any]) -> AdminUser:
        return AdminUser(
            id=row["id"],
            email=row["email"],
            first_name=row["first_name"],
            last_name=row["last_name"],
            role=row["role"],
            is_active=row["is_active"],
            is_blocked=row["is_blocked"],
            is_verified=row["is_verified"],
            language=row["language"],
            created_at=row["created_at"],
            last_login=row["last_login"],
        )

    async def list(self, limit: int, offset: int) -> list[AdminUser]:
        rows = sorted(self.w.users.values(), key=lambda r: r["created_at"], reverse=True)
        return [self._admin(row) for row in rows[offset : offset + limit]]

    async def count(self) -> int:
        return len(self.w.users)

    async def get(self, user_id: str) -> AdminUser | None:
        row = self.w.users.get(user_id)
        return None if row is None else self._admin(row)

    async def get_by_email(self, email: str) -> AdminUser | None:
        row = self.w.user_by_email(email)
        return None if row is None else self._admin(row)

    async def update(self, user_id: str, patch: Patch) -> AdminUser | None:
        row = self.w.users.get(user_id)
        if row is None:
            return None
        if patch.is_active is not None:
            row["is_active"] = patch.is_active
        if patch.is_blocked is not None:
            row["is_blocked"] = patch.is_blocked
        if patch.role is not None:
            row["role"] = patch.role
        return self._admin(row)

    async def upgrade_to_admin(self, user_id: str) -> None:
        self.w.users[user_id]["role"] = Role.ADMIN

    async def write_audit(
        self,
        actor_id: str,
        action: str,
        entity: str,
        entity_id: str,
        payload: str,
        ip: str,
        user_agent: str,
    ) -> None:
        self.w.audits.append(
            {
                "actor_id": actor_id,
                "action": action,
                "entity": entity,
                "entity_id": entity_id,
                "payload": payload,
            }
        )


class FakeWidgetStore:
    def __init__(self, world: World) -> None:
        self.w = world

    def _widget(self, row: dict[str, Any]) -> Widget:
        return Widget(
            id=row["id"],
            name=row["name"],
            kind=row["kind"],
            owner_user_id=row["owner_user_id"],
            created_at=row["created_at"],
        )

    async def insert(self, widget: Widget) -> None:
        self.w.widgets[widget.id] = {
            "id": widget.id,
            "name": widget.name,
            "kind": widget.kind,
            "owner_user_id": widget.owner_user_id,
            "created_at": widget.created_at,
        }

    async def list(self, owner_id: str, limit: int, offset: int) -> list[Widget]:
        rows = [row for row in self.w.widgets.values() if row["owner_user_id"] == owner_id]
        return [self._widget(row) for row in rows[offset : offset + limit]]

    async def get(self, widget_id: str, owner_id: str) -> Widget | None:
        row = self.w.widgets.get(widget_id)
        if row is None or row["owner_user_id"] != owner_id:
            return None
        return self._widget(row)

    async def update(
        self, widget_id: str, owner_id: str, name: str, kind: WidgetKind
    ) -> Widget | None:
        row = self.w.widgets.get(widget_id)
        if row is None or row["owner_user_id"] != owner_id:
            return None
        row.update(name=name, kind=kind)
        return self._widget(row)

    async def delete(self, widget_id: str, owner_id: str) -> bool:
        row = self.w.widgets.get(widget_id)
        if row is None or row["owner_user_id"] != owner_id:
            return False
        del self.w.widgets[widget_id]
        return True


class FakePublisher:
    """Records lifecycle events instead of fanning them out."""

    def __init__(self) -> None:
        self.events: list[tuple[str, dict[str, Any]]] = []

    async def publish(self, room: str, event: dict[str, Any]) -> None:
        self.events.append((room, event))


class FakeGoogle:
    """Verifies a ``local@ex.com`` credential into a matching identity.

    The real verifier checks an RS256 token against Google's JWKS; the fake
    treats the credential as the address it stands for and rejects anything
    that is not one, with the same codes and statuses as the real adapter.
    """

    def __init__(self, domain: str = "ex.com") -> None:
        self.configured = True
        self._domain = domain

    async def verify(self, credential: str) -> ExternalIdentity:
        if not self.configured:
            msg = "google_client_id is not configured"
            raise E(Code.AUTH_OAUTH_FAILED, msg, 503)
        local, _, domain = credential.partition("@")
        if not local or domain != self._domain:
            msg = "google identity token rejected"
            raise E(Code.AUTH_OAUTH_FAILED, msg, 401)
        return ExternalIdentity(
            provider="google",
            subject=f"google-{local}",
            email=credential,
            name=f"{local} Google",
        )


class FakeGithub:
    """Builds a canned consent URL and exchanges fixed codes.

    ``good-code`` provisions ``fresh@ex.com``; ``ada-code`` links the
    existing ``ada@ex.com`` account.
    """

    def __init__(self) -> None:
        self.configured = True
        self._codes = {
            "good-code": ExternalIdentity(
                provider="github", subject="gh-fresh", email="fresh@ex.com", name="Fresh GH"
            ),
            "ada-code": ExternalIdentity(
                provider="github", subject="gh-ada", email="ada@ex.com", name="Ada L"
            ),
        }

    def authorize_url(self, redirect_uri: str, state: str) -> str:
        query = {
            "client_id": "test-github-client-id",
            "redirect_uri": redirect_uri,
            "scope": "read:user user:email",
        }
        if state:
            query["state"] = state
        return f"https://github.com/login/oauth/authorize?{urlencode(query)}"

    async def exchange(self, code: str) -> ExternalIdentity:
        if not self.configured:
            msg = "github oauth is not configured"
            raise E(Code.AUTH_OAUTH_FAILED, msg, 503)
        identity = self._codes.get(code)
        if identity is None:
            msg = "github identity exchange failed"
            raise E(Code.AUTH_OAUTH_FAILED, msg, 401)
        return identity


def auth_header(identity: Identity, tokens: Any) -> dict[str, str]:
    """Mint a bearer header for a test caller."""
    return {"authorization": f"Bearer {tokens.mint_access(identity)}"}
