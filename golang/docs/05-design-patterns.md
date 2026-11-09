# Design Patterns (Go-Idiomatic)

The patterns this architecture leans on, stated as idiomatic Go — not a GoF
transliteration. Each entry: what it is, the Go way to express it, when it
earns its keep. Snippets are ≤10 lines on purpose.

## 1. Functional options

For platform components with a real default set and a growing option surface
(hub buffer size, worker poll interval, sender mode):

```go
func NewHub(opts ...Option) *Hub {
    h := &Hub{buf: 32, log: slog.Default()} // sane defaults
    for _, o := range opts { o(h) }
    return h
}
func WithBuffer(n int) Option { return func(h *Hub) { h.buf = n } }
```

Cost-benefit rule: fewer than three options or one construction path → a
plain `New(buf int, log *slog.Logger)`. Options buy named, additive
configuration; you pay for it only when callers diverge.

## 2. Ports defined by the consumer

The keystone ([Hexagonal Architecture](01-hexagonal-architecture.md)): the use
case writes the interface it needs, so it is exactly as wide as its use —
1–2 methods the adapter satisfies implicitly. The unit-test fake is 10 lines
and no mocking library enters the dependency graph.

## 3. Middleware chaining

Handlers wrap handlers with the stdlib signature; the chain composes inward.

```go
type Middleware func(http.Handler) http.Handler

func Chain(h http.Handler, m ...Middleware) http.Handler {
    for i := len(m) - 1; i >= 0; i-- { h = m[i](h) }
    return h
}
```

The skeleton's `platform/httpx` ships `Chain` plus the standard set — request
id, slog per-request, panic → 500 envelope, `authx.RequireAuth`,
`authx.RequireRole`. Each wrapper does one thing and passes the baton;
ordering is explicit at the mount point, not a global registry.

## 4. Small interfaces + zero-value structs

Structural typing makes "interface with one implementation" the cheapest seam
there is — the interface is free, the adapter is the production truth, the
fake is the second implementation. Constructors (`NewX(...) error`) return
ready values; the zero value is not the contract.

## 5. Composition over embedding, embedding over inheritance

There is no inheritance. Small structs compose; syntactic embedding is for
promoting one implementor's methods (e.g. embedding `*http.ServeMux` in a
route builder) — use it where the promoted methods keep meaning what they
say, and never to "extend" an adapter's behavior:

```go
type Server struct {
    *authx.Verifier          // promoted: Parse, Mint
    deps  Deps
}
```

## 6. Context as the cancel/error channel

Every I/O path takes `ctx` first and propagates it to ports and adapters.
`context.WithoutCancel` detaches fire-and-forget writes (audit survives a
hung-up client); `ctx.Done()` backs the worker poll loop and hub
subscriptions — shutdown is one cancel.

## 7. Error wrapping and `errors.Is/As`

Wrap with `%w` where context is added; test with `errors.Is`. One sentinel
per stable code keeps the envelope mapping dumb
([Errors & Config](10-error-config-observability.md)).

## 8. Table-driven tests

The only test framework this stack needs for unit tests: cases as a slice,
`name` strings for failure messages, `t.Run`. Fakes + table cases make the
golden pattern; godog covers behavior-level specs.

## 9. Null Object + offline guard

`platform/email` has two `Sender` implementations — `Console` (log the
rendered message) and `SMTP`. The selection function
`sender_from_config(offline, mode)` returns `Console` when
`<APP>_OFFLINE=true` before any socket type is constructed (INV-T7).

## 10. Strategy = a function value

Go's first-class functions retire most Strategy/Family-of-algorithms ceremony:
a `Backoff func(attempt int) time.Duration` is the retry policy; a
`CodeGenerator func(n int) Code` is the 6-digit issuer. Pass the function;
keep the struct for when state rides along.

## 11. Newtype discipline (light)

`type Role string` with a hierarchy (`rolex.Role`, the shipped example) —
newtypes where mixing values would be a real hazard. Clarity, not ceremony.

## 12. Interface pollution — the anti-pattern

Symptoms: interfaces declared next to the adapter "for architecture", 5-method
ports, `Deps` structs of interfaces nobody fakes. Cure: delete until a *test
or a second implementation* justifies the type. Consumer-declared consumer-
scoped — or gone.

## See also

- [Hexagonal Architecture](01-hexagonal-architecture.md)
- [Testing & CI](11-testing-and-ci.md)
- [Dependency Rules](04-dependency-rules.md)