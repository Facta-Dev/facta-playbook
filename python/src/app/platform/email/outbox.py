"""Durable email outbox: enqueue, claim, settle, purge.

The enqueue is transaction-aware by design: :func:`enqueue_email` only stages
the INSERT on the caller's session. The slice adapter that owns the business
write commits both rows in one transaction, so a crash cannot separate them.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import Any
from uuid import UUID, uuid4

from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncSession

_INSERT = text(
    """
    INSERT INTO email_jobs (id, to_email, template_key, payload, status, scheduled_at)
    VALUES (:id, :to_email, :template_key, CAST(:payload AS jsonb), 'queued', :now)
    """
)

_CLAIM = text(
    """
    WITH picked AS (
        SELECT id FROM email_jobs
        WHERE (status = 'queued' AND scheduled_at <= :now)
           OR (status = 'sending' AND scheduled_at <= :now - interval '5 minutes')
        ORDER BY scheduled_at
        LIMIT :limit
        FOR UPDATE SKIP LOCKED
    )
    UPDATE email_jobs j
    SET status = 'sending', scheduled_at = :now
    WHERE j.id IN (SELECT id FROM picked)
    RETURNING j.id, j.to_email, j.template_key, j.payload, j.status,
              j.retry_count, j.max_retries, j.scheduled_at, j.sent_at, j.last_error
    """
)

_COMPLETE = text(
    """
    UPDATE email_jobs
    SET status = 'sent', sent_at = :now, last_error = NULL
    WHERE id = :id
    """
)

_FAIL = text(
    """
    UPDATE email_jobs
    SET status = :status, retry_count = :retry_count, last_error = :cause,
        scheduled_at = :scheduled_at
    WHERE id = :id
    """
)

_PURGE = (
    (
        "sessions",
        "DELETE FROM sessions WHERE expires_at < :now OR is_active = false RETURNING id",
    ),
    (
        "email_verifications",
        "DELETE FROM email_verifications WHERE expires_at < :now RETURNING id",
    ),
    ("password_resets", "DELETE FROM password_resets WHERE expires_at < :now RETURNING id"),
)


@dataclass(frozen=True)
class EmailMessage:
    """What a use case hands to the outbox."""

    to: str
    template_key: str
    variables: dict[str, Any]
    locale: str = "en"


@dataclass(frozen=True)
class EmailJob:
    """A claimed outbox row."""

    id: UUID
    to: str
    template_key: str
    variables: dict[str, Any]
    status: str
    retry_count: int
    max_retries: int
    scheduled_at: datetime
    sent_at: datetime | None
    last_error: str | None


async def enqueue_email(session: AsyncSession, message: EmailMessage, now: datetime) -> None:
    """Stage the outbox row on the caller's transaction; never commits."""
    await session.execute(
        _INSERT,
        {
            "id": uuid4(),
            "to_email": message.to,
            "template_key": message.template_key,
            "payload": json.dumps({"variables": message.variables, "locale": message.locale}),
            "now": now,
        },
    )


def _payload(value: Any) -> dict[str, Any]:
    if isinstance(value, str):
        decoded: dict[str, Any] = json.loads(value)
        return decoded
    if isinstance(value, dict):
        return dict(value)
    return {}


async def claim(session: AsyncSession, now: datetime, limit: int) -> list[EmailJob]:
    """Claim due rows with a five-minute lease; the caller commits."""
    result = await session.execute(_CLAIM, {"now": now, "limit": limit})
    jobs: list[EmailJob] = []
    for row in result.mappings():
        raw = _payload(row["payload"])
        variables = raw.get("variables", {})
        jobs.append(
            EmailJob(
                id=row["id"],
                to=row["to_email"],
                template_key=row["template_key"],
                variables=dict(variables) if isinstance(variables, dict) else {},
                status=row["status"],
                retry_count=row["retry_count"],
                max_retries=row["max_retries"],
                scheduled_at=row["scheduled_at"],
                sent_at=row["sent_at"],
                last_error=row["last_error"],
            )
        )
    return jobs


async def complete(session: AsyncSession, job_id: UUID, now: datetime) -> None:
    """Mark a delivered job."""
    await session.execute(_COMPLETE, {"id": job_id, "now": now})


def backoff_seconds(attempt: int) -> int:
    """Exponential backoff in seconds, capped at 2^30 minutes."""
    return (1 << min(attempt, 30)) * 60


async def fail(session: AsyncSession, job: EmailJob, cause: str, now: datetime) -> None:
    """Requeue with backoff or dead-letter at ``max_retries``."""
    attempt = job.retry_count + 1
    status = "failed" if attempt >= job.max_retries else "queued"
    await session.execute(
        _FAIL,
        {
            "id": job.id,
            "status": status,
            "retry_count": attempt,
            "cause": cause[:500],
            "scheduled_at": now + timedelta(seconds=backoff_seconds(attempt)),
        },
    )


async def purge_expired(session: AsyncSession, now: datetime) -> dict[str, int]:
    """Delete expired sessions and one-shot codes; returns per-table counts."""
    counts: dict[str, int] = {}
    for name, statement in _PURGE:
        result = await session.execute(text(statement), {"now": now})
        counts[name] = len(result.all())
    return counts
