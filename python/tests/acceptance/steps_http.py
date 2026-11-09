"""Generic request and assertion steps shared by every feature.

Every step is registered without a keyword type, so a scenario may use the
same step as a Given, a When or a Then — Gherkin's ``And`` inherits the
previous keyword and the intent does not change.
"""

from __future__ import annotations

from typing import Any

from pytest_bdd import parsers, step

from app.shared.clockx import FixedClock
from tests.acceptance.support import Ctx, json_arr, json_map, sprint
from tests.acceptance.support import send as do_send
from tests.fakes import World


@step(parsers.re(r'I send "(?P<method>[A-Z]+)" "(?P<path>[^"]*)"'))
def send_plain(client: Any, ctx: Ctx, method: str, path: str) -> None:
    do_send(client, ctx, method, path)


@step(parsers.re(r'I send "(?P<method>[A-Z]+)" "(?P<path>[^"]*)" as (?P<who>[^:\s]+)'))
def send_as(client: Any, ctx: Ctx, method: str, path: str, who: str) -> None:
    do_send(client, ctx, method, path, "", who)


@step(parsers.re(r'I send "(?P<method>[A-Z]+)" "(?P<path>[^"]*)" with body:'))
def send_with_body(client: Any, ctx: Ctx, method: str, path: str, docstring: str = "") -> None:
    do_send(client, ctx, method, path, docstring.strip())


@step(parsers.re(r'I send "(?P<method>[A-Z]+)" "(?P<path>[^"]*)" as (?P<who>[^:\s]+) with body:'))
def send_as_with_body(
    client: Any, ctx: Ctx, method: str, path: str, who: str, docstring: str = ""
) -> None:
    do_send(client, ctx, method, path, docstring.strip(), who)


@step(parsers.re(r"the response status is (?P<status>\d+)"))
def response_status(ctx: Ctx, status: str) -> None:
    assert ctx.last.status_code == int(status), ctx.last.text


@step(parsers.re(r'the response code is "(?P<want>[^"]*)"'))
def response_code(ctx: Ctx, want: str) -> None:
    body = json_map(ctx.last.text)
    assert sprint(body.get("code")) == want, ctx.last.text


@step(parsers.re(r'the response field "(?P<field>[^"]*)" is "(?P<want>[^"]*)"'))
def response_field(ctx: Ctx, field: str, want: str) -> None:
    body = json_map(ctx.last.text)
    assert field in body, f'response has no field "{field}" ({ctx.last.text})'
    assert sprint(body[field]) == want, ctx.last.text


@step(
    parsers.re(r'the response field "(?P<field>[^"]*)" is "(?P<want>[^"]*)" on entry (?P<idx>\d+)')
)
def response_entry_field(ctx: Ctx, field: str, want: str, idx: str) -> None:
    entries = json_arr(ctx.last.text)
    index = int(idx)
    assert index < len(entries), f"want entry {index}, list has {len(entries)}"
    row = entries[index]
    assert field in row, f'entry {index} has no field "{field}" ({ctx.last.text})'
    assert sprint(row[field]) == want, ctx.last.text


@step(parsers.re(r"the response lists (?P<want>\d+) items?"))
def response_item_count(ctx: Ctx, want: str) -> None:
    body = json_map(ctx.last.text)
    items = body.get("items")
    items = items if isinstance(items, list) else []
    assert len(items) == int(want), ctx.last.text


@step(parsers.re(r"the list has (?P<want>\d+) entr(?:y|ies)"))
def response_list_count(ctx: Ctx, want: str) -> None:
    entries = json_arr(ctx.last.text)
    assert len(entries) == int(want), ctx.last.text


@step(parsers.re(r"the response carries an access token and a refresh token"))
def response_carries_tokens(ctx: Ctx) -> None:
    body = json_map(ctx.last.text)
    assert body.get("access_token"), ctx.last.text
    assert body.get("refresh_token"), ctx.last.text


@step(parsers.re(r'the response redirects to a URL that starts with "(?P<prefix>[^"]*)"'))
def response_redirects_to(ctx: Ctx, prefix: str) -> None:
    location = ctx.last.headers.get("location", "")
    assert location.startswith(prefix), location


@step(parsers.re(r"the response redirects carrying an access token"))
def response_redirects_with_tokens(ctx: Ctx) -> None:
    location = ctx.last.headers.get("location", "")
    assert "access_token=" in location, location
    assert "refresh_token=" in location, location


@step(parsers.re(r'the response redirects carrying error "(?P<code>[^"]*)"'))
def response_redirects_with_error(ctx: Ctx, code: str) -> None:
    location = ctx.last.headers.get("location", "")
    assert f"error={code}" in location, location


@step(
    parsers.re(
        r'(?P<count>\d+) "(?P<template>[a-z_]+)" emails? (?:is|are|was|were) '
        r'enqueued for "(?P<to>[^"]*)"'
    )
)
def emails_enqueued(world: World, count: str, template: str, to: str) -> None:
    got = world.count_emails(template, to)
    assert got == int(count), f'want {count} "{template}" email(s) for {to}, got {got}'


@step(parsers.re(r'I store the response field "(?P<field>[^"]*)" as (?P<name>[^:\s]+)'))
def store_response_field(ctx: Ctx, field: str, name: str) -> None:
    body = json_map(ctx.last.text)
    assert field in body, f'response has no field "{field}" ({ctx.last.text})'
    ctx.vars[name] = sprint(body[field])


@step(parsers.re(r"(?P<hours>\d+) hours pass"))
def hours_pass(clock: FixedClock, hours: str) -> None:
    clock.advance(int(hours) * 3600)


@step(parsers.re(r"(?P<minutes>\d+) minutes pass"))
def minutes_pass(clock: FixedClock, minutes: str) -> None:
    clock.advance(int(minutes) * 60)
