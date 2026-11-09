"""Profile picture and the social-login lookup indexes.

Revision ID: 0002_profiles
Revises: 0001_init
Create Date: 2026-01-01
"""

from __future__ import annotations

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import op

revision: str = "0002_profiles"
down_revision: str | None = "0001_init"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.add_column(
        "users",
        sa.Column("profile_picture", sa.Text(), nullable=False, server_default=""),
    )
    op.create_index("sessions_expiry", "sessions", ["expires_at"])
    op.create_index("email_jobs_status", "email_jobs", ["status"])


def downgrade() -> None:
    op.drop_index("email_jobs_status", table_name="email_jobs")
    op.drop_index("sessions_expiry", table_name="sessions")
    op.drop_column("users", "profile_picture")
