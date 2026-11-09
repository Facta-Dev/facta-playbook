"""Admin steps: the audit trail is shared state, not an HTTP response."""

from __future__ import annotations

from pytest_bdd import parsers, step

from tests.fakes import World


@step(parsers.re(r"(?P<count>\d+) audit entr(?:y|ies) (?:is|are|was|were) recorded"))
def audits_recorded(world: World, count: str) -> None:
    got = len(world.audits)
    assert got == int(count), f"want {count} audit entries, got {got}"
