# Auth architecture

Jakarta Security + MicroProfile JWT/OIDC. The tenant comes from the validated token claim; `@RolesAllowed` shapes the UX. The use case checks permission + ownership; the schema enforces `tenant_id` + RLS with a non-bypass role. Secrets live in a vault/Kubernetes Secret.
