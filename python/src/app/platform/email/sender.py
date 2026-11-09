"""Delivery adapters: console while offline, SMTP otherwise."""

from __future__ import annotations

import asyncio
import logging
import smtplib
from email.message import EmailMessage
from typing import Protocol

from app.platform.config import Settings

_log = logging.getLogger("app.email")


class Sender(Protocol):
    """One method, one message, one delivery attempt."""

    async def send(self, to: str, subject: str, body: str) -> None:
        """Deliver or raise; the outbox owns retries."""
        ...


class ConsoleSender:
    """Offline and development sender: logs the message and never dials."""

    async def send(self, to: str, subject: str, body: str) -> None:
        _log.info("email console to=%s subject=%s body=%s", to, subject, body)


class SmtpSender:
    """STARTTLS SMTP with a short timeout."""

    def __init__(self, settings: Settings) -> None:
        self._host = settings.smtp_host
        self._port = settings.smtp_port
        self._user = settings.smtp_user
        self._password = settings.smtp_password
        self._sender = settings.mail_from

    def _deliver(self, to: str, subject: str, body: str) -> None:
        message = EmailMessage()
        message["From"] = self._sender
        message["To"] = to
        message["Subject"] = subject
        message.set_content(body)
        with smtplib.SMTP(self._host, self._port, timeout=10) as client:
            client.starttls()
            if self._user:
                client.login(self._user, self._password)
            client.send_message(message)

    async def send(self, to: str, subject: str, body: str) -> None:
        await asyncio.to_thread(self._deliver, to, subject, body)


def select_sender(settings: Settings) -> Sender:
    """Offline or no host means the console sender, unconditionally."""
    if settings.offline or not settings.smtp_host:
        return ConsoleSender()
    return SmtpSender(settings)
