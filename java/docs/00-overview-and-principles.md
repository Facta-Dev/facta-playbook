# Overview and principles

`<app>` is a Jakarta EE 11 backend template: Java 21, Maven, CDI, JPA/Hibernate, Flyway, JAX-RS, PostgreSQL. The reference slice is `widgets`.

1. A slice owns its capability end to end.
2. Dependencies point inward; the domain is plain Java.
3. The use case owns the JTA transaction.
4. Entities never cross the API.
5. CDI construction is explicit.
6. Tenant isolation is in the use case and the schema (RLS).
7. The outbox commits with the domain row.
8. The app server is a deployment detail.
9. The server authorizes; the UI only shapes.
10. Extract a service only with measurements.

See `PLAYBOOK_BACKEND.md` for the binding rules.
