"""Composition root: the only module that knows every slice."""

from __future__ import annotations

import logging
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from dataclasses import dataclass, field
from typing import Any

from fastapi import FastAPI
from fastapi.responses import JSONResponse

from app.bin.middleware import install_middleware
from app.features.adminusers.module import register as register_adminusers
from app.features.adminusers.postgres import PgAdminUserStore
from app.features.adminusers.usecase import AdminUserService
from app.features.authoauth.module import register as register_authoauth
from app.features.authoauth.postgres import PgOAuthStore
from app.features.authoauth.usecase import OAuthService
from app.features.authpassword.module import register as register_authpassword
from app.features.authpassword.postgres import PgPasswordStore
from app.features.authpassword.usecase import PasswordService
from app.features.authsession.module import register as register_authsession
from app.features.authsession.postgres import PgSessionStore
from app.features.authsession.usecase import SessionService
from app.features.authsignup.module import register as register_authsignup
from app.features.authsignup.postgres import PgSignupStore
from app.features.authsignup.usecase import SignupService
from app.features.realtime.module import register as register_realtime
from app.features.widgets.module import register as register_widgets
from app.features.widgets.postgres import PgWidgetStore
from app.features.widgets.usecase import WidgetService
from app.platform.authx import Passwords, Tokens
from app.platform.config import Settings
from app.platform.db import Database
from app.platform.envelope import error_body, install_error_handlers
from app.platform.logging import configure_logging
from app.platform.oauthx import GithubOAuth, GoogleVerifier
from app.platform.realtime.hub import Hub
from app.shared.apperr import Code
from app.shared.clockx import Clock, SystemClock
from app.shared.idgen import SystemIds

_log = logging.getLogger("app.api")


@dataclass
class Deps:
    """Everything the app needs, so tests can swap in fakes."""

    settings: Settings
    signup: SignupService
    session: SessionService
    password: PasswordService
    oauth: OAuthService
    adminusers: AdminUserService
    widgets: WidgetService
    tokens: Tokens
    hub: Hub = field(default_factory=Hub)
    database: Database | None = None


def wire(settings: Settings) -> Deps:
    """Build the real adapters and services."""
    database = Database.connect(settings)
    clock: Clock = SystemClock()
    ids = SystemIds()
    hub = Hub()
    passwords = Passwords(settings.password_pepper)
    tokens = Tokens(settings, clock)

    signup = SignupService(
        PgSignupStore(database),
        passwords,
        ids,
        clock,
        code_ttl_hours=settings.verify_code_ttl_hours,
        max_attempts=settings.verify_max_attempts,
    )
    session = SessionService(
        PgSessionStore(database),
        passwords,
        tokens,
        ids,
        clock,
        refresh_ttl_hours=settings.refresh_ttl_hours,
    )
    password = PasswordService(
        PgPasswordStore(database),
        passwords,
        ids,
        clock,
        reset_ttl_minutes=settings.reset_ttl_minutes,
        rate_limit=settings.reset_rate_limit,
        max_attempts=settings.verify_max_attempts,
    )
    oauth = OAuthService(
        PgOAuthStore(database),
        GoogleVerifier(settings),
        GithubOAuth(settings),
        tokens,
        ids,
        clock,
        callback_url=settings.github_callback_url,
        frontend_success=settings.frontend_success,
        offline=settings.offline,
        refresh_ttl_hours=settings.refresh_ttl_hours,
    )
    adminusers = AdminUserService(PgAdminUserStore(database), settings.allowed_admin_emails)
    widgets = WidgetService(PgWidgetStore(database), hub, ids, clock)
    return Deps(
        settings=settings,
        signup=signup,
        session=session,
        password=password,
        oauth=oauth,
        adminusers=adminusers,
        widgets=widgets,
        tokens=tokens,
        hub=hub,
        database=database,
    )


def build_app(deps: Deps) -> FastAPI:
    """Assemble the app from a dependency set; no globals, no import-time I/O."""

    @asynccontextmanager
    async def lifespan(_app: FastAPI) -> AsyncIterator[None]:
        yield
        if deps.database is not None:
            await deps.database.close()

    app = FastAPI(
        title=deps.settings.jwt_issuer,
        version="0.1.0",
        lifespan=lifespan,
        docs_url="/docs",
        redoc_url=None,
    )
    app.state.tokens = deps.tokens
    app.state.hub = deps.hub

    install_error_handlers(app)
    install_middleware(app, deps.settings)

    register_authsignup(app, deps.signup)
    register_authsession(app, deps.session)
    register_authpassword(app, deps.password)
    register_authoauth(app, deps.oauth)
    register_adminusers(app, deps.adminusers)
    register_widgets(app, deps.widgets)
    register_realtime(app, deps.hub, deps.tokens)

    @app.get("/healthz")
    async def healthz() -> dict[str, str]:
        return {"status": "ok"}

    @app.get("/readyz")
    async def readyz() -> Any:
        if deps.database is None:
            return {"status": "ok", "database": "not_configured"}
        ok, detail = await deps.database.readyz()
        if not ok:
            return JSONResponse(status_code=503, content=error_body(Code.INTERNAL, detail))
        return {"status": "ok", "database": detail}

    return app


def create_app() -> FastAPI:
    """Entrypoint helper: load settings, wire, build."""
    settings = Settings()
    configure_logging(settings.log_level)
    return build_app(wire(settings))


def main() -> None:
    """Serve with uvicorn; the worker is a separate process."""
    import uvicorn

    settings = Settings()
    uvicorn.run(
        build_app(wire(settings)),
        host="0.0.0.0",
        port=8000,
        log_config=None,
    )


if __name__ == "__main__":
    main()
