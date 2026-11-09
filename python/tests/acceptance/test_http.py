"""HTTP contract checks that do not need a scenario."""

from __future__ import annotations

from typing import Any

import pytest
from starlette.testclient import TestClient
from starlette.websockets import WebSocketDisconnect

from app.bin.api import Deps, build_app
from app.platform.authx import Identity
from app.shared.apperr import Code
from app.shared.rolex import Role

FAILING_REQUESTS = [
    ("GET", "/nope", None, 404),
    ("POST", "/api/v1/auth/login", {"email": "not-an-email", "password": "x"}, 422),
    ("GET", "/api/v1/auth/me", None, 401),
    ("GET", "/api/v1/admin/users", None, 401),
    ("POST", "/api/v1/auth/refresh", {"refresh_token": "n" * 64}, 401),
]


def test_healthz(client: Any) -> None:
    response = client.get("/healthz")
    assert response.status_code == 200
    assert response.json() == {"status": "ok"}


def test_readyz_without_database(client: Any) -> None:
    response = client.get("/readyz")
    assert response.status_code == 200
    assert response.json()["database"] == "not_configured"


def test_error_envelope_for_unknown_login(client: Any) -> None:
    response = client.post(
        "/api/v1/auth/login", json={"email": "ghost@example.com", "password": "whatever-123"}
    )
    assert response.status_code == 401
    assert response.json() == {
        "detail": "invalid email or password",
        "code": "AUTH_INVALID_CREDENTIALS",
    }


def test_unknown_route_uses_the_envelope(client: Any) -> None:
    response = client.get("/nope")
    assert response.status_code == 404
    assert response.json()["code"] == "NOT_FOUND"


def test_validation_errors_use_the_envelope(client: Any) -> None:
    response = client.post("/api/v1/auth/signup", json={"email": "not-an-email"})
    assert response.status_code == 422
    assert response.json()["code"] == "VALIDATION_FAILED"


def test_security_headers_and_request_id(client: Any) -> None:
    response = client.get("/healthz", headers={"x-request-id": "test-123"})
    assert response.headers["x-request-id"] == "test-123"
    assert response.headers["x-content-type-options"] == "nosniff"
    assert response.headers["x-frame-options"] == "DENY"


@pytest.mark.parametrize(("method", "path", "payload", "status"), FAILING_REQUESTS)
def test_every_failure_uses_the_same_envelope(
    client: Any, method: str, path: str, payload: Any, status: int
) -> None:
    response = client.request(method, path, json=payload)
    assert response.status_code == status, response.text
    body = response.json()
    assert set(body) == {"detail", "code"}, body
    assert body["code"] in {code.value for code in Code}, body
    assert isinstance(body["detail"], str) and body["detail"], body


def test_unhandled_errors_use_the_envelope(
    client: Any, deps: Deps, monkeypatch: pytest.MonkeyPatch
) -> None:
    async def boom(*_args: Any, **_kwargs: Any) -> Any:
        msg = "boom"
        raise RuntimeError(msg)

    monkeypatch.setattr(deps.session, "login", boom)
    response = client.post(
        "/api/v1/auth/login", json={"email": "ghost@example.com", "password": "whatever-123"}
    )
    assert response.status_code == 500
    assert response.json() == {"detail": "internal server error", "code": "INTERNAL"}


def test_websocket_auth_failure_closes_with_4001(deps: Deps) -> None:
    app = build_app(deps)
    with TestClient(app) as socket_client:
        with socket_client.websocket_connect("/api/v1/ws?token=not-a-token") as socket:
            with pytest.raises(WebSocketDisconnect) as closed:
                socket.receive_json()
    assert closed.value.code == 4001


def test_websocket_hello_carries_the_user_id(deps: Deps) -> None:
    token = deps.tokens.mint_access(
        Identity(user_id="user-1", email="a@example.com", role=Role.USER)
    )
    app = build_app(deps)
    with TestClient(app) as socket_client:
        with socket_client.websocket_connect(f"/api/v1/ws?token={token}") as socket:
            assert socket.receive_json() == {"type": "hello", "user_id": "user-1"}
