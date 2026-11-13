# Request lifecycle

1. Correlation/security filter.
2. JAX-RS resource parses and validates the DTO.
3. The resource calls one use case (`@Transactional`).
4. The use case loads through ports, applies the rule, writes and appends the outbox row.
5. The resource maps `Result` to HTTP + the envelope.
6. The outbox worker (or a JMS listener) sends the event; the consumer is idempotent.
