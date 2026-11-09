"""Login, refresh and logout steps."""

from __future__ import annotations

import json
from typing import Any

from pytest_bdd import parsers, step

from tests.acceptance.support import Ctx, json_map, pair
from tests.acceptance.support import send as do_send


@step(parsers.re(r'I log in as "(?P<email>[^"]*)" with password "(?P<pw>[^"]*)"'))
def log_in(client: Any, ctx: Ctx, email: str, pw: str) -> None:
    body = json.dumps({"email": email, "password": pw})
    do_send(client, ctx, "POST", "/api/v1/auth/login", body)


@step(
    parsers.re(r'I log in as "(?P<email>[^"]*)" with password "(?P<pw>[^"]*)" (?P<count>\d+) times')
)
def log_in_repeatedly(client: Any, ctx: Ctx, email: str, pw: str, count: str) -> None:
    body = json.dumps({"email": email, "password": pw})
    for _ in range(int(count)):
        do_send(client, ctx, "POST", "/api/v1/auth/login", body)


@step(
    parsers.re(
        r'I log in as "(?P<email>[^"]*)" with password "(?P<pw>[^"]*)" '
        r"and remember the pair as (?P<label>[^:\s]+)"
    )
)
def log_in_and_remember(client: Any, ctx: Ctx, email: str, pw: str, label: str) -> None:
    body = json.dumps({"email": email, "password": pw})
    do_send(client, ctx, "POST", "/api/v1/auth/login", body)
    assert ctx.last.status_code == 200, ctx.last.text
    payload = json_map(ctx.last.text)
    access = str(payload.get("access_token") or "")
    refresh = str(payload.get("refresh_token") or "")
    assert access and refresh, ctx.last.text
    ctx.pairs[label] = {"access": access, "refresh": refresh}


@step(
    parsers.re(
        r'I log in as "(?P<email>[^"]*)" with password "(?P<pw>[^"]*)" '
        r'and remember the pair as (?P<label>[^:\s]+) from the address "(?P<ip>[^"]*)"'
    )
)
def log_in_and_remember_from_ip(
    client: Any, ctx: Ctx, email: str, pw: str, label: str, ip: str
) -> None:
    body = json.dumps({"email": email, "password": pw})
    do_send(client, ctx, "POST", "/api/v1/auth/login", body, "", ip)
    assert ctx.last.status_code == 200, ctx.last.text
    payload = json_map(ctx.last.text)
    access = str(payload.get("access_token") or "")
    refresh = str(payload.get("refresh_token") or "")
    assert access and refresh, ctx.last.text
    ctx.pairs[label] = {"access": access, "refresh": refresh}


@step(parsers.re(r'I refresh using the token "(?P<label>[^"]+)"'))
def refresh(client: Any, ctx: Ctx, label: str) -> None:
    """Rotation retires the presented token, so the pair stays as it was."""
    body = json.dumps({"refresh_token": pair(ctx, label)["refresh"]})
    do_send(client, ctx, "POST", "/api/v1/auth/refresh", body)


@step(parsers.re(r'I log out with the token "(?P<label>[^"]+)"'))
def log_out(client: Any, ctx: Ctx, label: str) -> None:
    current = pair(ctx, label)
    body = json.dumps({"refresh_token": current["refresh"]})
    do_send(client, ctx, "POST", "/api/v1/auth/logout", body, token=current["access"])


@step(parsers.re(r'I remember the refresh token of "(?P<label>[^"]*)" as (?P<name>[^:\s]+)'))
def remember_refresh_token(ctx: Ctx, label: str, name: str) -> None:
    ctx.vars[name] = pair(ctx, label)["refresh"]
