"""Widget event-stream steps.

The Python stream frames carry the event name inside the JSON payload
(``data: {"type": "widget.created", ...}``) instead of the SSE ``event:``
field, so the assertion reads the payload.
"""

from __future__ import annotations

import json
from typing import Any

from pytest_bdd import parsers, step

from tests.acceptance.support import Ctx, bearer


@step(parsers.re(r'I open the event stream for "(?P<who>[^"]*)"'))
def open_event_stream(client: Any, ctx: Ctx, who: str) -> None:
    headers = bearer(ctx, who)
    headers["accept"] = "text/event-stream"
    manager = client.stream("GET", "/api/v1/widgets/events", headers=headers)
    response = manager.__enter__()
    assert response.status_code == 200, response.text
    ctx.streams[who] = (manager, response)


@step(parsers.re(r'the event stream of "(?P<who>[^"]*)" shows a "(?P<want>[^"]*)" event'))
def event_stream_shows(ctx: Ctx, who: str, want: str) -> None:
    stream = ctx.streams.get(who)
    assert stream is not None, f'no open event stream for "{who}"'
    _manager, response = stream
    for line in response.iter_lines():
        if not line.startswith("data: "):
            continue
        payload = json.loads(line[len("data: ") :])
        assert payload.get("type") == want, f"want a {want!r} event, got {payload!r}"
        return
    msg = f'no event arrived on the stream of "{who}"'
    raise AssertionError(msg)
