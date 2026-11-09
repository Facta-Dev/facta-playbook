"""Worker entrypoint: the outbox drain and the janitor."""

from __future__ import annotations

import asyncio
import logging
import signal

from app.platform.config import Settings
from app.platform.db import Database
from app.platform.email.sender import select_sender
from app.platform.email.worker import janitor_loop, outbox_loop
from app.platform.logging import configure_logging
from app.shared.clockx import SystemClock

_log = logging.getLogger("app.worker")


async def run(settings: Settings) -> None:
    """Own both loops; a single stop event shuts them down."""
    database = Database.connect(settings)
    sender = select_sender(settings)
    clock = SystemClock()
    stop = asyncio.Event()

    loop = asyncio.get_running_loop()
    for signum in (signal.SIGINT, signal.SIGTERM):
        loop.add_signal_handler(signum, stop.set)

    tasks = [
        asyncio.create_task(outbox_loop(database, sender, settings, clock, stop)),
        asyncio.create_task(janitor_loop(database, settings, clock, stop)),
    ]
    _log.info("worker started")
    await stop.wait()
    for task in tasks:
        task.cancel()
    await asyncio.gather(*tasks, return_exceptions=True)
    await database.close()
    _log.info("worker stopped")


def main() -> None:
    """Process entrypoint."""
    settings = Settings()
    configure_logging(settings.log_level)
    asyncio.run(run(settings))


if __name__ == "__main__":
    main()
