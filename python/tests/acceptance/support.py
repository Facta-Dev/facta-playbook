"""Shared state and helpers for the acceptance step modules.

The steps stay synchronous: the app runs under uvicorn in a background
thread (see ``tests/conftest.py``), so every request goes through a plain
``httpx.Client``.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from typing import Any


@dataclass
class Ctx:
    """Per-scenario harness state."""

    last: Any = None
    pairs: dict[str, dict[str, str]] = field(default_factory=dict)
    vars: dict[str, str] = field(default_factory=dict)
    streams: dict[str, Any] = field(default_factory=dict)

    def close(self) -> None:
        """Release the open event streams."""
        for manager, _response in self.streams.values():
            manager.__exit__(None, None, None)
        self.streams.clear()


def sprint(value: Any) -> str:
    """Render a JSON scalar the way the feature files spell it."""
    if isinstance(value, bool):
        return "true" if value else "false"
    if value is None:
        return "null"
    return str(value)


def json_map(body: str) -> dict[str, Any]:
    """Decode a response body into an object."""
    if body == "":
        return {}
    value = json.loads(body)
    if not isinstance(value, dict):
        msg = f"response body is not a JSON object: {body}"
        raise AssertionError(msg)
    return value


def json_arr(body: str) -> list[Any]:
    """Decode a response body into an array."""
    if body == "":
        return []
    value = json.loads(body)
    if not isinstance(value, list):
        msg = f"response body is not a JSON array: {body}"
        raise AssertionError(msg)
    return value


def pair(ctx: Ctx, label: str) -> dict[str, str]:
    """The remembered pair behind a step label, or a helpful error."""
    found = ctx.pairs.get(label)
    if found is None:
        msg = f'no logged-in session "{label}": log in with "remember the pair as {label}" first'
        raise AssertionError(msg)
    return found


def bearer(ctx: Ctx, label: str) -> dict[str, str]:
    """The Authorization header for a remembered pair."""
    return {"authorization": f"Bearer {pair(ctx, label)['access']}"}


def expand(ctx: Ctx, text: str) -> str:
    """Substitute ``{{name}}`` placeholders in paths and bodies."""
    for key, value in ctx.vars.items():
        text = text.replace("{{" + key + "}}", value)
    return text


def send(
    client: Any,
    ctx: Ctx,
    method: str,
    path: str,
    body: str = "",
    who: str = "",
    ip: str = "",
    token: str = "",
) -> Any:
    """Issue one request against the running app and record the exchange."""
    headers: dict[str, str] = {}
    if body:
        headers["content-type"] = "application/json"
    if who:
        headers.update(bearer(ctx, who))
    if token:
        headers["authorization"] = f"Bearer {token}"
    if ip:
        headers["x-forwarded-for"] = ip
    payload = expand(ctx, body) if body else None
    ctx.last = client.request(
        method,
        expand(ctx, path),
        content=payload.encode() if payload is not None else None,
        headers=headers,
    )
    return ctx.last


def wrong_code(code: str) -> str:
    """Change one digit of a code: same length, definitely a different value."""
    return ("1" if code.startswith("0") else "0") + code[1:]
