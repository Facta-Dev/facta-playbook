# Hexagonal architecture

| Layer | Contains | May depend on |
|---|---|---|
| domain | entities, value objects, ports | Java SE, shared |
| application | use cases, transaction boundary | domain, ports |
| adapters | JPA, JMS, HTTP clients | application, domain |
| presentation | JAX-RS, DTOs | application |

The ports belong to the consumer: `WidgetRepository` lives in application, `JpaWidgetRepository` implements it in adapters. `ArchiveWidgetService` depends on the port, not on `EntityManager`.
