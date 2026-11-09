"""The one error envelope and the request helpers shared by every router."""

from __future__ import annotations

import logging

from fastapi import FastAPI, Request
from fastapi.exceptions import RequestValidationError
from fastapi.responses import JSONResponse
from starlette.exceptions import HTTPException as StarletteHTTPException

from app.shared.apperr import AppError, Code

_log = logging.getLogger("app.error")


def error_body(code: Code | str, detail: str) -> dict[str, str]:
    """The only error shape clients ever see."""
    value = code.value if isinstance(code, Code) else code
    return {"detail": detail, "code": value}


def install_error_handlers(app: FastAPI) -> None:
    """Map domain, validation, and framework failures onto the envelope."""

    @app.exception_handler(AppError)
    async def _app_error(_request: Request, exc: AppError) -> JSONResponse:
        return JSONResponse(status_code=exc.status, content=error_body(exc.code, exc.detail))

    @app.exception_handler(RequestValidationError)
    async def _validation(_request: Request, exc: RequestValidationError) -> JSONResponse:
        return JSONResponse(
            status_code=422,
            content=error_body(Code.VALIDATION_FAILED, "request validation failed"),
        )

    @app.exception_handler(StarletteHTTPException)
    async def _http(_request: Request, exc: StarletteHTTPException) -> JSONResponse:
        code = Code.NOT_FOUND if exc.status_code == 404 else Code.INTERNAL
        detail = exc.detail if isinstance(exc.detail, str) else "request failed"
        return JSONResponse(status_code=exc.status_code, content=error_body(code, detail))

    @app.exception_handler(Exception)
    async def _unhandled(_request: Request, exc: Exception) -> JSONResponse:
        # Log the trace, return only the envelope: internals never reach clients.
        _log.exception("unhandled error", exc_info=exc)
        return JSONResponse(
            status_code=500, content=error_body(Code.INTERNAL, "internal server error")
        )


def client_ip(request: Request) -> str:
    """Trust the proxy chain only for the first hop; fall back to the socket."""
    forwarded = request.headers.get("x-forwarded-for")
    if forwarded:
        return forwarded.split(",")[0].strip()
    return request.client.host if request.client is not None else "unknown"


def user_agent(request: Request) -> str:
    """Bounded user agent string for the session row."""
    return request.headers.get("user-agent", "")[:256]


def pagination(page: int, page_size: int) -> tuple[int, int]:
    """Clamp caller-supplied pagination to a safe window."""
    safe_page = max(1, page)
    safe_size = min(max(1, page_size), 100)
    return safe_page, safe_size
