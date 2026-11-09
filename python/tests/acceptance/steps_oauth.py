"""Social sign-in steps: Google One Tap and the GitHub flow."""

from __future__ import annotations

import json
from typing import Any

from pytest_bdd import parsers, step

from tests.acceptance.support import Ctx
from tests.acceptance.support import send as do_send
from tests.fakes import FakeGithub, FakeGoogle


@step(parsers.re(r'"(?P<email>[^"]*)" signs in with a Google credential'))
def google_sign_in(client: Any, ctx: Ctx, email: str) -> None:
    body = json.dumps({"credential": email})
    do_send(client, ctx, "POST", "/api/v1/auth/google", body)


@step(parsers.re(r'"(?P<email>[^"]*)" signs in with an invalid Google credential'))
def google_sign_in_invalid(client: Any, ctx: Ctx, email: str) -> None:
    body = json.dumps({"credential": "not-a-credential"})
    do_send(client, ctx, "POST", "/api/v1/auth/google", body)


@step(parsers.re(r"the google provider is not configured"))
def google_not_configured(google: FakeGoogle) -> None:
    google.configured = False


@step(parsers.re(r"the github provider is disabled"))
def github_disabled(github: FakeGithub) -> None:
    github.configured = False
