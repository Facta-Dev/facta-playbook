# Design Patterns Catalogue

The core patterns a hexagonal Rust backend leans on. Each is generic; the Rust
realization follows.

## 1. Repository

Decouple persistence from business rules. A trait declares the contract; a `Pg*`
adapter implements it with sqlx. Consumers depend on the trait.

```rust
#[async_trait]
pub trait WidgetRepository: Send + Sync + 'static {
    async fn find(&self, id: WidgetId) -> Result<Option<Widget>>;
}
```

## 2. Ports & Adapters

See [01 — Hexagonal Architecture](01-hexagonal-architecture.md). Ports are
traits; adapters are the concretions wired only at the root.

## 3. Composition Root

One place (`<app>-api/src/state.rs`) constructs every concrete adapter and
injects it as `Arc<dyn Trait>`. Nothing depends on the root; the root depends on
everything inward.

## 4. Strategy

A port with multiple implementations is a strategy. `Mailer` has `NoopMailer`
(tests) and `ProviderMailer` (prod). The composition root picks which.

## 5. Facade

A use-case `Service` struct is a facade over several repos: callers see one
`create()` method; the service coordinates validate → persist → notify across
`Arc<dyn Trait>` collaborators.

## 6. Observer + Null Object

Outbound collaborators (publisher, mailer) are `Arc<dyn Trait>`. The Null
Object (`NoopMailer`, `NoopPublisher`) is the double that drops every side
effect, letting a full flow run in a test with no external world.

## 7. Caching Proxy

A port can be wrapped by a caching adapter that implements the same trait,
checks a cache, and falls through to the real adapter. Same trait → caller
unchanged. Example: a poll→cache→publish gateway.

## 8. Newtype / Typed-ID

A newtype around `Uuid` makes a whole class of bugs unrepresentable:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WidgetId(pub uuid::Uuid);
```

`fn f(id: WidgetId)` cannot be handed a `UserId`. The compiler is your first
test layer.

## See also

- [Hexagonal architecture](01-hexagonal-architecture.md)
- [Crate topology](03-crate-topology.md)
- [Testing & CI](11-testing-and-ci.md)