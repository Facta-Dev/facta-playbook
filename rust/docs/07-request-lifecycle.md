# Request Lifecycle

End-to-end trace of an HTTP request through a hexagonal axum workspace.

## 1. The path

```mermaid
sequenceDiagram
    autonumber
    participant C as Client
    participant MW as Middleware (auth, rate-limit)
    participant H as Handler (axum)
    participant UC as Use-case Service
    participant PORT as Port trait
    participant ADP as Pg adapter
    C->>MW: POST /api/v1/<scope>
    MW->>MW: authenticate + rate-limit + authorize
    MW->>H: extract request body
    H->>UC: service.create(input)
    UC->>UC: validate (domain rules)
    UC->>PORT: repo.upsert(entity)
    PORT->>ADP: query_as! + fetch
    ADP-->>PORT: row
    PORT-->>UC: entity
    UC-->>H: Result<entity>
    H-->>C: 201 JSON
```

## 2. The layers crossed

1. **Transport** — axum `serve` accepts the TCP connection.
2. **Middleware** — `tower-http` (tracing, CORS) + auth/rate-limit extractors.
3. **Extractor** — axum `Json<T>` deserializes the body into a request DTO.
4. **Handler** — validates at the HTTP boundary, maps to a domain input, calls
   the use-case service. Maps `core::Error` to an HTTP status.
5. **Use-case** — business rules over `Arc<dyn Trait>` repos. No HTTP types.
6. **Port** — the trait call; pure-domain signature.
7. **Adapter** — the `Pg*` adapter runs `query_as!` (compile-time checked),
   maps `sqlx::Error` → `core::Error`, returns a domain entity.

## 3. Error path

```mermaid
sequenceDiagram
    participant H as Handler
    participant UC as Use-case
    participant ADP as Adapter
    ADP->>UC: Err(sqlx::Error::RowNotFound) mapped to Error::NotFound
    UC->>H: Err(Error::NotFound)
    H->>H: match Error → 404 JSON
```

The handler is the **only** place that knows about HTTP status codes. The
use-case and adapter speak `core::Error` exclusively. One `match` in the handler
maps the domain error enum to responses.

## 4. Read vs write

- **Read** (`GET`): handler → service → `repo.find` → entity → serialize. No
  mutation; safe to cache.
- **Write** (`POST/PUT`): handler → service → validate → `repo.upsert` → entity.
  The adapter wraps the write in a transaction if it spans multiple statements.

## See also

- [Hexagonal architecture](01-hexagonal-architecture.md)
- [Error & observability](10-error-config-observability.md)
- [Persistence & migrations](08-persistence-and-migrations.md)