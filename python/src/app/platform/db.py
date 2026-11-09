"""The one async engine and session factory for the process."""

from __future__ import annotations

from collections.abc import AsyncIterator
from dataclasses import dataclass

from sqlalchemy import text
from sqlalchemy.exc import SQLAlchemyError
from sqlalchemy.ext.asyncio import (
    AsyncEngine,
    AsyncSession,
    async_sessionmaker,
    create_async_engine,
)

from app.platform.config import Settings


@dataclass
class Database:
    """Owns the engine; hands out short-lived sessions."""

    engine: AsyncEngine
    session_factory: async_sessionmaker[AsyncSession]

    @classmethod
    def connect(cls, settings: Settings) -> Database:
        """Create the engine without opening a socket."""
        engine = create_async_engine(settings.database_url, pool_pre_ping=True)
        factory = async_sessionmaker(engine, expire_on_commit=False)
        return cls(engine=engine, session_factory=factory)

    def session(self) -> AsyncSession:
        """A new session; the caller owns commit/rollback."""
        return self.session_factory()

    async def readyz(self) -> tuple[bool, str]:
        """Check the database and that migrations have been applied."""
        try:
            async with self.session() as session:
                await session.execute(text("SELECT 1"))
                result = await session.execute(
                    text("SELECT version_num FROM alembic_version LIMIT 1")
                )
                row = result.first()
        except SQLAlchemyError as exc:
            return False, f"database unavailable: {type(exc).__name__}"
        if row is None:
            return False, "alembic_version is empty"
        return True, str(row[0])

    async def close(self) -> None:
        """Dispose the pool during shutdown."""
        await self.engine.dispose()


async def session_scope(database: Database) -> AsyncIterator[AsyncSession]:
    """Yield a session that commits on success and rolls back on error."""
    async with database.session() as session:
        try:
            yield session
            await session.commit()
        except Exception:
            await session.rollback()
            raise
