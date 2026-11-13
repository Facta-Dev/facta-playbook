# Design patterns

- **Use case**: `ArchiveWidgetService` with `@Transactional`, ports and `Result`.
- **Adapter**: JPA entity mapping, `@Version`, fetch joins, tenant predicate.
- **Outbox**: domain row + outbox row in one transaction; a worker sends and marks.
- **Resource**: parse, validate, call one use case, map the Result.
- **Error envelope**: one `ExceptionMapper` per family.
