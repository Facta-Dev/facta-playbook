"""Outbox drain and janitor loops. Each loop owns its sessions and clock."""

from __future__ import annotations

import asyncio
import logging

from app.platform.config import Settings
from app.platform.db import Database
from app.platform.email.outbox import claim, complete, fail, purge_expired
from app.platform.email.sender import Sender
from app.platform.email.templates import render
from app.shared.clockx import Clock

_log = logging.getLogger("app.email.worker")


async def drain_once(
    database: Database,
    sender: Sender,
    settings: Settings,
    clock: Clock,
) -> int:
    """Claim one batch, deliver each row, settle it. Returns rows handled."""
    now = clock.now()
    async with database.session() as session:
        jobs = await claim(session, now, settings.outbox_batch)
        await session.commit()
    for job in jobs:
        try:
            rendered = render(job.template_key, job.variables)
            await sender.send(job.to, rendered.subject, rendered.body)
        except Exception as exc:
            _log.warning("email failed job=%s error=%s", job.id, type(exc).__name__, exc_info=True)
            async with database.session() as session:
                await fail(session, job, f"{type(exc).__name__}: {exc}", clock.now())
                await session.commit()
        else:
            async with database.session() as session:
                await complete(session, job.id, clock.now())
                await session.commit()
    return len(jobs)


async def outbox_loop(
    database: Database,
    sender: Sender,
    settings: Settings,
    clock: Clock,
    stop: asyncio.Event,
) -> None:
    """Drain forever until ``stop`` is set."""
    while not stop.is_set():
        try:
            handled = await drain_once(database, sender, settings, clock)
        except Exception:
            _log.exception("outbox drain failed")
            handled = 0
        if handled == 0:
            try:
                await asyncio.wait_for(stop.wait(), timeout=settings.outbox_poll_seconds)
            except TimeoutError:
                continue


async def janitor_loop(
    database: Database,
    settings: Settings,
    clock: Clock,
    stop: asyncio.Event,
) -> None:
    """Purge expired sessions and one-shot codes on a slow cadence."""
    while not stop.is_set():
        try:
            async with database.session() as session:
                counts = await purge_expired(session, clock.now())
                await session.commit()
            _log.info("janitor purged=%s", counts)
        except Exception:
            _log.exception("janitor sweep failed")
        try:
            await asyncio.wait_for(stop.wait(), timeout=settings.janitor_poll_seconds)
        except TimeoutError:
            continue
