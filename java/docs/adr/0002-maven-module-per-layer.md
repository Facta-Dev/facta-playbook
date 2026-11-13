# ADR 0002 — Maven module per layer

Accepted. Each slice has domain/application/adapters/presentation modules; the reactor and ArchUnit enforce the dependency direction. The alternative (one module with packages) was rejected because it cannot fail at compile time.
