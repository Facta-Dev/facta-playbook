"""Shared fixtures: real uvicorn, fake adapters.

The server runs in a background thread on an ephemeral port, so the BDD
steps stay synchronous (pytest-bdd does not await async step functions).
"""

from __future__ import annotations

import threading
import time
from collections.abc import Iterator
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import httpx
import pytest
import uvicorn
from argon2 import PasswordHasher

from app.bin.api import Deps, build_app
from app.features.adminusers.usecase import AdminUserService
from app.features.authoauth.usecase import OAuthService
from app.features.authpassword.usecase import PasswordService
from app.features.authsession.usecase import SessionService
from app.features.authsignup.usecase import SignupService
from app.features.widgets.usecase import WidgetService
from app.platform import authx
from app.platform.authx import Passwords, Tokens
from app.platform.config import Settings
from app.platform.realtime.hub import Hub
from app.shared.clockx import FixedClock
from tests.acceptance.support import Ctx
from tests.fakes import (
    FakeAdminUserStore,
    FakeGithub,
    FakeGoogle,
    FakeOAuthStore,
    FakePasswordStore,
    FakeSessionStore,
    FakeSignupStore,
    FakeWidgetStore,
    SequenceIds,
    World,
)

FEATURES = Path(__file__).resolve().parents[1] / "specs" / "features"


@pytest.fixture(autouse=True)
def fast_password_hashing(monkeypatch: pytest.MonkeyPatch) -> None:
    """Run argon2 at test cost: the shipped parameters cost seconds per hash.

    The tiers exercise the behavior around the KDF, not the KDF's cost.
    """
    monkeypatch.setattr(
        authx, "_hasher", PasswordHasher(time_cost=1, memory_cost=8192, parallelism=1)
    )


@pytest.fixture
def settings() -> Settings:
    return Settings(
        jwt_secret="example-jwt-secret-0123456789abcdef",
        database_url="postgresql+asyncpg://app:app@127.0.0.1:5432/app",
        offline=False,
        allowed_admin_emails=["admin-boost@ex.com"],
    )


@pytest.fixture
def world() -> World:
    return World()


@pytest.fixture
def clock() -> FixedClock:
    return FixedClock(datetime.now(UTC))


@pytest.fixture
def passwords() -> Passwords:
    return Passwords("")


@pytest.fixture
def google() -> FakeGoogle:
    return FakeGoogle()


@pytest.fixture
def github() -> FakeGithub:
    return FakeGithub()


@pytest.fixture
def hub() -> Hub:
    return Hub()


@pytest.fixture
def deps(
    settings: Settings,
    world: World,
    clock: FixedClock,
    passwords: Passwords,
    google: FakeGoogle,
    github: FakeGithub,
    hub: Hub,
) -> Deps:
    ids = SequenceIds(world)
    tokens = Tokens(settings, clock)
    oauth = OAuthService(
        FakeOAuthStore(world),
        google,
        github,
        tokens,
        ids,
        clock,
        callback_url=settings.github_callback_url,
        frontend_success=settings.frontend_success,
        offline=settings.offline,
        refresh_ttl_hours=settings.refresh_ttl_hours,
    )
    return Deps(
        settings=settings,
        signup=SignupService(
            FakeSignupStore(world),
            passwords,
            ids,
            clock,
            code_ttl_hours=settings.verify_code_ttl_hours,
            max_attempts=settings.verify_max_attempts,
        ),
        session=SessionService(
            FakeSessionStore(world),
            passwords,
            tokens,
            ids,
            clock,
            refresh_ttl_hours=settings.refresh_ttl_hours,
        ),
        password=PasswordService(
            FakePasswordStore(world),
            passwords,
            ids,
            clock,
            reset_ttl_minutes=settings.reset_ttl_minutes,
            rate_limit=settings.reset_rate_limit,
            max_attempts=settings.verify_max_attempts,
        ),
        oauth=oauth,
        adminusers=AdminUserService(FakeAdminUserStore(world), settings.allowed_admin_emails),
        widgets=WidgetService(FakeWidgetStore(world), hub, ids, clock),
        tokens=tokens,
        hub=hub,
    )


@pytest.fixture
def client(deps: Deps) -> Iterator[Any]:
    """Serve the real app under uvicorn in a thread on an ephemeral port."""
    app = build_app(deps)
    config = uvicorn.Config(app, host="127.0.0.1", port=0, log_config=None, lifespan="on")
    server = uvicorn.Server(config)
    thread = threading.Thread(target=server.run, daemon=True)
    thread.start()
    while not server.started:
        time.sleep(0.01)
    port = server.servers[0].sockets[0].getsockname()[1]
    with httpx.Client(base_url=f"http://127.0.0.1:{port}", timeout=5.0) as http:
        yield http
    server.should_exit = True
    thread.join(timeout=5)


@pytest.fixture
def ctx() -> Iterator[Ctx]:
    """Per-scenario state; the open event streams are released at the end."""
    state = Ctx()
    yield state
    state.close()
