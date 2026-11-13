# Memory — `<app>` Java / Jakarta EE skeleton

- Architecture: hexagonal + vertical slices; the `widgets` slice is the
  reference. The full contract is `PLAYBOOK_BACKEND.md` (J2EE).
- Runtime: Jakarta EE 11, Java 21, Maven, WAR on WildFly/Payara/Open Liberty.
- Persistence: JPA 3.2/Hibernate 7, Flyway is the schema authority,
  `hbm2ddl.auto=validate`.
- Transactions: JTA `@Transactional` on the use case; no XA; the outbox is the
  courier.
- Tenant isolation: `tenant_id` on every table, PostgreSQL RLS with a
  non-bypass application role.
- Presentation: JAX-RS, DTO records, the `{error:{code,message,
  requiredPermission,correlationId}}` envelope.
- Security: Jakarta Security + MicroProfile JWT/OIDC; secrets in a vault.
- Gates: `make ci`, `make evals`, `make verify`; Checkstyle in CI.
- Known gaps: the realtime/SSE adapter and the two-tenant integration suite
  are not shipped in this template; add them with the feature.
