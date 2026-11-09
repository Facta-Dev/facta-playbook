"""Password reset and change steps."""

from __future__ import annotations

import json
from typing import Any

from pytest_bdd import parsers, step

from tests.acceptance.support import Ctx, pair, wrong_code
from tests.acceptance.support import send as do_send
from tests.fakes import World


@step(parsers.re(r'I ask to reset the password for "(?P<email>[^"]*)"'))
def ask_to_reset(client: Any, ctx: Ctx, email: str) -> None:
    body = json.dumps({"email": email})
    do_send(client, ctx, "POST", "/api/v1/auth/forgot-password", body)


@step(parsers.re(r'I asked to reset the password for "(?P<email>[^"]*)" (?P<count>\d+) times'))
def ask_to_reset_repeatedly(client: Any, ctx: Ctx, email: str, count: str) -> None:
    body = json.dumps({"email": email})
    for _ in range(int(count)):
        do_send(client, ctx, "POST", "/api/v1/auth/forgot-password", body)


@step(parsers.re(r'I requested a reset code for "(?P<email>[^"]*)"'))
def request_reset_code(client: Any, ctx: Ctx, email: str) -> None:
    body = json.dumps({"email": email})
    do_send(client, ctx, "POST", "/api/v1/auth/forgot-password", body)
    assert ctx.last.status_code == 202, ctx.last.text


@step(parsers.re(r'I validate "(?P<email>[^"]*)" with the latest reset code'))
def validate_latest_code(client: Any, ctx: Ctx, world: World, email: str) -> None:
    code = world.latest_email_code("reset_password", email)
    body = json.dumps({"email": email, "code": code})
    do_send(client, ctx, "POST", "/api/v1/auth/validate-code", body)


@step(parsers.re(r'I validate "(?P<email>[^"]*)" with a wrong reset code'))
def validate_wrong_code(client: Any, ctx: Ctx, world: World, email: str) -> None:
    code = wrong_code(world.latest_email_code("reset_password", email))
    body = json.dumps({"email": email, "code": code})
    do_send(client, ctx, "POST", "/api/v1/auth/validate-code", body)


@step(
    parsers.re(
        r'I reset the password for "(?P<email>[^"]*)" using the latest reset code '
        r'and password "(?P<pw>[^"]*)"'
    )
)
def reset_password(client: Any, ctx: Ctx, world: World, email: str, pw: str) -> None:
    code = world.latest_email_code("reset_password", email)
    body = json.dumps({"email": email, "code": code, "new_password": pw})
    do_send(client, ctx, "POST", "/api/v1/auth/reset-password", body)


@step(
    parsers.re(
        r'I change the password of (?P<label>[^:\s]+) from "(?P<current>[^"]*)" '
        r'to "(?P<fresh>[^"]*)"'
    )
)
def change_password(client: Any, ctx: Ctx, label: str, current: str, fresh: str) -> None:
    current_pair = pair(ctx, label)
    body = json.dumps({"current_password": current, "new_password": fresh})
    do_send(client, ctx, "POST", "/api/v1/auth/change-password", body, token=current_pair["access"])
