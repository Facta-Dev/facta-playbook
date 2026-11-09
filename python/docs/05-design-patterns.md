# Design Patterns (Python-Idiomatic)

The patterns this architecture leans on, stated as idiomatic Python — not a
GoF transliteration. Each entry: what it is, the Python way to express it,
when it earns its keep. Snippets are ≤10 lines on purpose.

## 1. Defaults over options bags

For platform components with a real default set (hub queue size, worker poll
interval, token TTL), a plain default parameter beats an options object:

```python
class Hub:
    def __init__(self, queue_size: int = 32) -> None:
        self._queue_size = queue_size
```

Cost-benefit rule: reach for keyword arguments only when the third boolean
arrives or construction paths diverge — named parameters are cheap; ceremony
is not. `Settings` (pydantic) is the exception: there the field set _is_ the
configuration surface.

## 2. Ports defined by the consumer

The keystone ([Hexagonal Architecture](01-hexagonal-architecture.md)): the use
case writes the `Protocol` it needs, so it is exactly as wide as its use — a
few methods the adapter satisfies structurally. The unit-test fake is 10 lines
and no mocking library enters the dependency graph.

## 3. Dependency chains (FastAPI `Depends`)

FastAPI owns the chain shape: cross-cutting wrappers are Starlette middleware
(`app.add_middleware(CORSMiddleware)`), per-route guards are dependencies
that read left to right:

```python
Admin = Annotated[Identity, Depends(require_role(Role.ADMIN))]


@router.patch("/admin/users/{user_id}")
async def update_user(user_id: str, body: UpdateRequest, actor: Admin) -> AdminUserResponse: ...
```

The skeleton's `src/app/bin/middleware.py` ships the standard set — request
id, security headers, origin-allowlist CORS — and `app/platform/authx.py`
owns `require_auth`/`require_role`. Each dependency does one thing; ordering
is explicit in the handler's signature, not a global registry.

## 4. Small protocols + plain classes

`typing.Protocol` is structural, so "protocol with one implementation" is the
cheapest seam there is — the protocol is free, the adapter is the production
truth, the fake is the second implementation. Prefer plain classes or
functions over inheritance hierarchies; constructors return ready objects,
the partially-initialized value is never the contract.

## 5. Composition over inheritance

Inheritance chains couple you to a base class's every future change; the
skeleton has none. Compose: a use case takes its dependencies as constructor
arguments and adapters are one class each. Modules export small named
functions or dataclasses — the class boundary exists where state rides along
(an adapter holding an engine).

## 6. `asyncio` cancellation as the shutdown channel

Cancellation is an event, not a context parameter: the worker builds one
`asyncio.Event`, wired to SIGINT/SIGTERM, and every loop polls it, so
shutdown is one event:

```python
stop = asyncio.Event()
loop.add_signal_handler(signal.SIGTERM, stop.set)
await asyncio.wait_for(stop.wait(), timeout=settings.outbox_poll_seconds)
```

Streaming paths (WS, SSE) unbind their hub subscription in a `finally:`
block — the disconnect _is_ the cancellation.

## 7. Exception chaining

Wrap with `raise E(code, detail, status) from exc` where context is added;
test on the stable `code`, never on the message. One code per failure mode
keeps the envelope mapping dumb
([Errors & Config](10-error-config-observability.md)). `AppError` in
`app/shared/apperr.py` is the single shape crossing every layer unchanged.

## 8. Table-driven tests (pytest)

The only test machinery unit tests need: cases as a list, names in the
failure message. `pytest.mark.parametrize` covers repeated branches;
`pytest.raises(AppError)` asserts on the stable `code`, never on the message.
Fakes + table cases make the golden pattern; the `specs/features` pytest-bdd
suite covers behavior-level contracts.

## 9. Null Object + offline guard

`app/platform/email` has two `Sender` implementations — `ConsoleSender` (log
the rendered message) and `SmtpSender`. The selection function
`select_sender(settings)` returns the console sender when
`<APP>_OFFLINE=true` or the SMTP host is empty, before any socket type is
constructed (INV-T7), and the `Settings` validator rejects SMTP credentials
outright while offline.

## 10. Strategy = a callable

First-class functions retire most Strategy/Family-of-algorithms ceremony: a
`Clock` protocol plus `SystemClock`/`FixedClock` is the time policy; an
`IdSource` protocol plus `SystemIds` is the id policy. Pass the object; keep
the class for when state rides along.

## 11. `StrEnum` discipline (light)

`Role` in `app/shared/rolex.py` is a `StrEnum` —
`guest ⊂ user ⊂ moderator ⊂ admin` — with a `_RANK` table and an
`at_least(role, floor)` helper. `Code` in `app/shared/apperr.py` is the same
shape for error codes. Literal values where mixing them would be a real
hazard, plus a rank table under them. Clarity, not branded-type ceremony.

## 12. Protocol pollution — the anti-pattern

Symptoms: protocols declared next to the adapter "for architecture", five-
method ports, `Deps` objects of protocols nobody fakes. Cure: delete until a
_test or a second implementation_ justifies the type. Consumer-declared,
consumer-scoped — or gone.

## See also

- [Hexagonal Architecture](01-hexagonal-architecture.md)
- [Testing & CI](11-testing-and-ci.md)
- [Dependency Rules](04-dependency-rules.md)
