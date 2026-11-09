"""Use cases against in-memory adapters; no sockets, no Postgres."""

from __future__ import annotations

from datetime import UTC, datetime

import pytest

from app.features.adminusers.domain import Patch
from app.features.adminusers.usecase import AdminUserService
from app.features.authpassword.usecase import PasswordService
from app.features.authsession.domain import TokenPair
from app.features.authsession.usecase import SessionService
from app.features.authsignup.usecase import SignupService
from app.features.widgets.usecase import WidgetService
from app.platform.authx import Identity, Passwords, Tokens
from app.platform.config import Settings
from app.shared.apperr import AppError, Code
from app.shared.clockx import FixedClock
from app.shared.rolex import Role
from tests.fakes import (
    FakeAdminUserStore,
    FakePasswordStore,
    FakePublisher,
    FakeSessionStore,
    FakeSignupStore,
    FakeWidgetStore,
    SequenceIds,
    World,
)

PASSWORD = "correct-horse-battery"


@pytest.fixture
def settings() -> Settings:
    return Settings(
        jwt_secret="example-jwt-secret-0123456789abcdef",
        database_url="postgresql+asyncpg://app:app@127.0.0.1:5432/app",
        offline=True,
    )


@pytest.fixture
def world() -> World:
    return World()


@pytest.fixture
def clock() -> FixedClock:
    return FixedClock(datetime.now(UTC))


@pytest.fixture
def passwords() -> Passwords:
    return Passwords("")


@pytest.fixture
def tokens(settings: Settings, clock: FixedClock) -> Tokens:
    return Tokens(settings, clock)


@pytest.fixture
def signup(world: World, passwords: Passwords, clock: FixedClock) -> SignupService:
    return SignupService(FakeSignupStore(world), passwords, SequenceIds(world), clock)


@pytest.fixture
def session(
    world: World, passwords: Passwords, tokens: Tokens, clock: FixedClock
) -> SessionService:
    return SessionService(FakeSessionStore(world), passwords, tokens, SequenceIds(world), clock)


@pytest.fixture
def password(world: World, passwords: Passwords, clock: FixedClock) -> PasswordService:
    return PasswordService(FakePasswordStore(world), passwords, SequenceIds(world), clock)


async def _verified_login(
    signup: SignupService, session: SessionService, world: World, email: str
) -> TokenPair:
    await signup.signup(email, PASSWORD)
    await signup.verify(email, world.latest_job_code())
    return await session.login(email, PASSWORD, "127.0.0.1", "pytest")


async def test_signup_stages_user_code_and_job_together(
    signup: SignupService, world: World
) -> None:
    result = await signup.signup("new@example.com", PASSWORD)
    assert result.code_queued is True
    assert len(world.users) == 1
    assert len(world.codes) == 1
    assert len(world.jobs) == 1
    assert world.jobs[0]["template_key"] == "verify_email"


async def test_verify_wrong_code_burns_an_attempt(signup: SignupService, world: World) -> None:
    await signup.signup("new@example.com", PASSWORD)
    with pytest.raises(AppError) as caught:
        await signup.verify("new@example.com", "000000")
    assert caught.value.code is Code.AUTH_CODE_INVALID
    assert world.codes[0]["attempts"] == 1


async def test_signup_rejects_a_verified_email(signup: SignupService, world: World) -> None:
    await signup.signup("new@example.com", PASSWORD)
    await signup.verify("new@example.com", world.latest_job_code())
    with pytest.raises(AppError) as caught:
        await signup.signup("new@example.com", PASSWORD)
    assert caught.value.code is Code.AUTH_EMAIL_TAKEN


async def test_login_gates_run_in_order(
    signup: SignupService, session: SessionService, world: World
) -> None:
    await _verified_login(signup, session, world, "blocked@example.com")
    blocked = world.user_by_email("blocked@example.com")
    assert blocked is not None
    blocked["is_blocked"] = True
    with pytest.raises(AppError) as caught:
        await session.login("blocked@example.com", PASSWORD, "ip", "ua")
    assert caught.value.code is Code.AUTH_ACCOUNT_BLOCKED
    assert caught.value.status == 403


async def test_refresh_rotation_rejects_replay(
    signup: SignupService, session: SessionService, world: World
) -> None:
    pair = await _verified_login(signup, session, world, "rot@example.com")
    fresh = await session.refresh(pair.refresh_token)
    assert fresh.refresh_token != pair.refresh_token
    with pytest.raises(AppError) as caught:
        await session.refresh(pair.refresh_token)
    assert caught.value.code is Code.AUTH_INVALID_TOKEN


async def test_forgot_is_silent_for_unknown_email(password: PasswordService, world: World) -> None:
    await password.forgot("ghost@example.com")
    assert world.jobs == []


async def test_reset_changes_the_password(
    signup: SignupService, password: PasswordService, session: SessionService, world: World
) -> None:
    await _verified_login(signup, session, world, "reset@example.com")
    await password.forgot("reset@example.com")
    code = world.latest_job_code()
    await password.reset("reset@example.com", code, "new-password-123")
    await session.login("reset@example.com", "new-password-123", "ip", "ua")


async def test_reset_code_is_not_consumed_by_validate(
    signup: SignupService, password: PasswordService, session: SessionService, world: World
) -> None:
    await _verified_login(signup, session, world, "val@example.com")
    await password.forgot("val@example.com")
    code = world.latest_job_code()
    await password.validate("val@example.com", code)
    await password.reset("val@example.com", code, "new-password-123")


async def test_expired_reset_code_is_rejected(
    signup: SignupService,
    password: PasswordService,
    session: SessionService,
    world: World,
    clock: FixedClock,
) -> None:
    await _verified_login(signup, session, world, "late@example.com")
    await password.forgot("late@example.com")
    code = world.latest_job_code()
    clock.advance(2 * 60 * 60)
    with pytest.raises(AppError) as caught:
        await password.reset("late@example.com", code, "new-password-123")
    assert caught.value.code is Code.AUTH_CODE_EXPIRED


async def test_admin_cannot_edit_a_peer(world: World) -> None:
    admin = world.add_user("root@example.com", role=Role.ADMIN)
    peer = world.add_user("peer@example.com", role=Role.ADMIN)
    service = AdminUserService(FakeAdminUserStore(world), [])
    actor = Identity(user_id=admin["id"], email=admin["email"], role=Role.ADMIN)
    with pytest.raises(AppError) as caught:
        await service.update(actor, peer["id"], Patch(True, False, None), "ip", "ua")
    assert caught.value.code is Code.FORBIDDEN


async def test_admin_update_is_audited(world: World) -> None:
    admin = world.add_user("root@example.com", role=Role.ADMIN)
    target = world.add_user("user@example.com")
    service = AdminUserService(FakeAdminUserStore(world), [])
    actor = Identity(user_id=admin["id"], email=admin["email"], role=Role.ADMIN)
    await service.update(actor, target["id"], Patch(False, None, None), "ip", "ua")
    assert world.users[target["id"]]["is_active"] is False
    assert world.audits[-1]["action"] == "admin_user.update"


async def test_upgrade_requires_the_whitelist(world: World) -> None:
    world.add_user("root@example.com")
    service = AdminUserService(FakeAdminUserStore(world), ["root@example.com"])
    with pytest.raises(AppError) as caught:
        await service.upgrade_to_admin("other@example.com")
    assert caught.value.code is Code.FORBIDDEN
    result = await service.upgrade_to_admin("root@example.com")
    assert result.upgraded is True


async def test_widgets_are_owner_scoped(world: World, clock: FixedClock) -> None:
    owner = world.add_user("owner@example.com")
    intruder = world.add_user("intruder@example.com")
    publisher = FakePublisher()
    service = WidgetService(FakeWidgetStore(world), publisher, SequenceIds(world), clock)

    widget = await service.create(owner["id"], "gear", "cog")
    with pytest.raises(AppError) as caught:
        await service.get(widget.id, intruder["id"])
    assert caught.value.code is Code.WIDGET_NOT_FOUND
    assert (publisher.events[-1][0], publisher.events[-1][1]["type"]) == (
        f"user:{owner['id']}",
        "widget.created",
    )


async def test_widget_kind_is_validated(world: World, clock: FixedClock) -> None:
    owner = world.add_user("owner@example.com")
    service = WidgetService(FakeWidgetStore(world), None, SequenceIds(world), clock)
    with pytest.raises(AppError) as caught:
        await service.create(owner["id"], "gear", "sprocket-plus")
    assert caught.value.code is Code.VALIDATION_FAILED
