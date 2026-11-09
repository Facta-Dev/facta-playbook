"""Signup, verification and account-seeding steps."""

from __future__ import annotations

import json
from typing import Any

from pytest_bdd import parsers, step

from app.platform.authx import Passwords
from app.shared.rolex import Role
from tests.acceptance.support import Ctx, wrong_code
from tests.acceptance.support import send as do_send
from tests.fakes import World

PASSWORD = "s3cure-pass-123"


@step(parsers.re(r'I sign up "(?P<email>[^"]*)" with password "(?P<pw>[^"]*)"'))
def sign_up(client: Any, ctx: Ctx, email: str, pw: str) -> None:
    body = json.dumps({"email": email, "password": pw})
    do_send(client, ctx, "POST", "/api/v1/auth/signup", body)


@step(parsers.re(r'"(?P<email>[^"]*)" has signed up but not verified'))
def signed_up_but_not_verified(client: Any, ctx: Ctx, email: str) -> None:
    body = json.dumps({"email": email, "password": PASSWORD})
    do_send(client, ctx, "POST", "/api/v1/auth/signup", body)


@step(
    parsers.re(
        r"an? (?P<mod>ADMIN |verified |unverified |inactive |blocked )?user "
        r'"(?P<email>[^"]*)" with password "(?P<pw>[^"]*)"'
    )
)
def seed_user(world: World, passwords: Passwords, mod: str | None, email: str, pw: str) -> None:
    """Seed an account straight into the store, so the ids stay deterministic."""
    state = (mod or "").strip()
    world.add_user(
        email,
        password_hash=passwords.hash(pw),
        role=Role.ADMIN if state == "ADMIN" else Role.USER,
        verified=state != "unverified",
        active=state != "inactive",
        blocked=state == "blocked",
    )


@step(parsers.re(r'I verify "(?P<email>[^"]*)" with the latest emailed code'))
def verify_with_latest_code(client: Any, ctx: Ctx, world: World, email: str) -> None:
    code = world.latest_email_code("verify_email", email)
    body = json.dumps({"email": email, "code": code})
    do_send(client, ctx, "POST", "/api/v1/auth/verify", body)


@step(parsers.re(r'I verify "(?P<email>[^"]*)" with a wrong code'))
def verify_with_wrong_code(client: Any, ctx: Ctx, world: World, email: str) -> None:
    code = wrong_code(world.latest_email_code("verify_email", email))
    body = json.dumps({"email": email, "code": code})
    do_send(client, ctx, "POST", "/api/v1/auth/verify", body)


@step(parsers.re(r'I resend verification for "(?P<email>[^"]*)"'))
def resend_verification(client: Any, ctx: Ctx, email: str) -> None:
    body = json.dumps({"email": email})
    do_send(client, ctx, "POST", "/api/v1/auth/verify/resend", body)
