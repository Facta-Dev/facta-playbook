# Testing and CI

| Level | Tooling |
|---|---|
| Unit | JUnit 5 + fake ports |
| CDI | Weld SE |
| Adapters | Testcontainers PostgreSQL/Artemis |
| API | REST Assured |
| Architecture | ArchUnit + Enforcer |
| Isolation | two-tenant fixtures |

CI runs `mvn verify`, `make evals` and the `.claude` linters. A release blocker is a test, not a comment.
