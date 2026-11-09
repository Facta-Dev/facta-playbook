# Design Patterns (TypeScript-Idiomatic)

The patterns this architecture leans on, stated as idiomatic TypeScript — not
a GoF transliteration. Each entry: what it is, the TS way to express it, when
it earns its keep. Snippets are ≤10 lines on purpose.

## 1. Defaults over options bags

For platform components with a real default set (`newHub(buf)` buffer size,
worker poll interval, sender mode), a plain positional parameter with a
default beats an options object:

```ts
export const newHub = (buf = 16): Hub => ({ buf, subs: new Map() });
```

Cost-benefit rule: reach for an options object/destructuring only when the
third boolean arrives or construction paths diverge — named parameters are
cheap; ceremony is not.

## 2. Ports defined by the consumer

The keystone ([Hexagonal Architecture](01-hexagonal-architecture.md)): the use
case writes the interface it needs, so it is exactly as wide as its use — a
few methods the adapter satisfies structurally. The unit-test fake is 10 lines
and no mocking library enters the dependency graph.

## 3. Hooks and preHandler chains

Fastify owns the chain shape: cross-cutting wrappers are `onRequest` hooks
(`app.addHook("onRequest", requestID)`), per-route guards are `preHandler`
arrays that read left to right:

```ts
app.put(
  "/api/v1/admin/users/:id",
  { preHandler: [requireAuth(v, cookie), requireRole(rolex.Admin)] },
  h.update,
);
```

The skeleton's `src/bin/middleware.ts` ships the standard set — request id,
security headers, origin-allowlist CORS — and `platform/authx` owns
`requireAuth`/`requireRole`. Each hook does one thing; ordering is explicit
at the mount point, not a global registry.

## 4. Small interfaces + plain classes

TypeScript interfaces are structural, so "interface with one implementation"
is the cheapest seam there is — the interface is free, the adapter is the
production truth, the fake is the second implementation. Prefer plain classes
or closures over inheritance hierarchies; constructors return ready objects,
the partially-initialized value is never the contract.

## 5. Composition over inheritance

`extends` chains couple you to a base class's every future change; the
skeleton has none. Compose: a use case takes its dependencies as constructor
arguments and adapters are one class each. Modules export small named
functions — the class boundary exists where state rides along (an adapter
holding a Pool).

## 6. AbortSignal as the cancel/error channel

Cancellation is a signal, not a context parameter — but the discipline is the
Go sibling's: every I/O path takes the signal through ports to adapters. The
API root and the worker build one `AbortController` each, wired to
SIGTERM/SIGINT; `signal.addEventListener("abort", …)` unbinds hub
subscriptions and timers, so shutdown is one abort:

```ts
const ctl = new AbortController();
process.once("SIGTERM", () => ctl.abort());
await runEmailWorker(new Outbox(pool), sender, pollMs, batch, ctl.signal);
```

## 7. Error cause chains

Wrap with `new Error(msg, { cause })` where context is added; test with
`err.cause` or `instanceof`. One stable `code` per failure mode keeps the
envelope mapping dumb ([Errors & Config](10-error-config-observability.md)).
`#shared/apperr`'s `from()` is the single coercion from unknown thrown values
into an `AppError`.

## 8. Table-driven tests (vitest `it.each`)

The only test machinery unit tests need: cases as an array, names in the
failure message. Fakes + table cases make the golden pattern; the
`specs/features` cucumber suite covers behavior-level contracts.

## 9. Null Object + offline guard

`platform/email` has two `Sender` implementations — `Console` (log the
rendered message) and `SMTP`. The selection function
`selectSender(cfg, log)` returns `Console` when `<APP>_OFFLINE=true` before
any socket type is constructed (INV-T7).

## 10. Strategy = a function value

First-class functions retire most Strategy/Family-of-algorithms ceremony:
a `Backoff = (attempt: number): number` is the retry policy; a
`CodeGenerator = (bytes: number): string` is the 6-digit issuer. Pass the
function; keep the class for when state rides along.

## 11. Literal-union discipline (light)

`type Role = "GUEST" | "USER" | "MODERATOR" | "ADMIN"` (`rolex.Role`, the
shipped example) — literal unions where mixing values would be a real hazard,
plus a `Rank` table under them. Clarity, not branded-type ceremony.

## 12. Interface pollution — the anti-pattern

Symptoms: interfaces declared next to the adapter "for architecture", five-
method ports, `Deps` objects of interfaces nobody fakes. Cure: delete until a
_test or a second implementation_ justifies the type. Consumer-declared,
consumer-scoped — or gone.

## See also

- [Hexagonal Architecture](01-hexagonal-architecture.md)
- [Testing & CI](11-testing-and-ci.md)
- [Dependency Rules](04-dependency-rules.md)
