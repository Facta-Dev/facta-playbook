# ADR 0003 — JPA + transactional outbox

Accepted. JPA 3.2/Hibernate 7 for persistence; Flyway is the schema authority; the outbox row commits with the domain row and a worker delivers it. XA/distributed transactions were rejected.
