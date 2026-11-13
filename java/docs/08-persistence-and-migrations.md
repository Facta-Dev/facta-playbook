# Persistence and migrations

JPA 3.2/Hibernate 7. Entities live in adapters; `@Version` on every mutable aggregate; fetch joins/projections inside the transaction. Flyway is the only schema authority; `hbm2ddl.auto=validate` in deployed profiles. Every table has `tenant_id`, standard timestamps and RLS. Expand/contract for zero downtime.
