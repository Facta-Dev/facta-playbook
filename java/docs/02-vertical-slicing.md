# Vertical slicing

A slice is one capability: `widgets` for the reference CRUD/archive flow, `identity` for auth, `tenants` for membership. Each slice has its own domain, application, adapters, presentation and migration prefix.

Split when the capabilities change independently; do not split by technical layer. A cross-slice read goes through a port/contracts, never a table join.
