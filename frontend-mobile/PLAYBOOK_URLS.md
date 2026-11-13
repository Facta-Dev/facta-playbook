# PLAYBOOK_URLS.md — URL / route naming playbook

Route-naming contract for Heimdall's backend API and SPA routes, adapted from
the enterprise multi-tenant URL convention. Owner-provided template
(2026-11-06), normalized to Heimdall's actors. The one rule that carries the
whole file: **the user-type segment never authorizes** — it is an experience
namespace; permissions are validated by session, token and RBAC in the backend.

## Enterprise template

### Frontend

```text
/{userType}/{module}/{resource}
/{userType}/{module}/{resource}/{resourceId}
/{userType}/{module}/{resource}/{resourceId}/{subresource}
```

Examples:

```text
/app/funnels/leads/{leadId}
/admin/tenants/{tenantId}/users
/admin/billing/invoices/{invoiceId}
/partner/campaigns/{campaignId}
```

### API

```text
/api/{version}/{userType}/{module}/{resource}
/api/{version}/{userType}/{module}/{resource}/{resourceId}
/api/{version}/{userType}/{module}/{resource}/{resourceId}/{subresource}
```

Examples:

```text
GET  /api/v1/app/funnels/leads/{leadId}
GET  /api/v1/admin/tenants/{tenantId}/users
GET  /api/v1/partner/campaigns/{campaignId}/calls
POST /api/v1/admin/tenants/{tenantId}/users/{userId}/suspend
```

## The {userType} segment

Use product roles/personas, never authentication details.

| Segment | Use | Example |
|---|---|---|
| `/app` | End-user of the client company (seller/employee console) | `/app/funnels/leads` |
| `/admin` | Platform's own console or super admin | `/admin/tenants` |
| `/owner` | Account/tenant owner | `/owner/settings` |
| `/manager` | Manager with extended permissions | `/manager/team` |
| `/agent` | Operational user — sales agent, support | `/agent/inbox` |
| `/partner` | Partner, agency, reseller | `/partner/clients` |
| `/public` | Session-less routes | `/public/pricing` |

Never use `/user` as the generic segment: `/app` is the end-user experience
and `/admin` is reserved for administration. This avoids redundant URLs like
`/user/user/leads`.

## Authorization rules

- `{userType}` defines **experience and navigation**, not security.
- The backend must validate: user identity from the token; the active
  tenant/organization; granular role and permissions; resource ownership.
- Never trust the `/admin` prefix to grant access.
- A user with multiple roles holds an active session role, or the backend
  resolves permissions per resource.

```text
GET /api/v1/admin/tenants/{tenantId}/users
Authorization: Bearer <token>
X-Tenant-Id: tenant_123
```

Even though the route says `/admin`, the backend must check the token belongs
to a user with `tenant.users.read` for that `tenantId`.

Heimdall mapping (premise in `MEMORY.md`): the admin configures API keys,
creates employee users and monitors one funnel per employee — so the admin
console lives under `/admin`, each employee's funnel view under `/app/funnels`,
and every cross-employee or cross-tenant read is an admin permission checked
in the backend, never a URL assumption.

## Multi-tenant structure

Include the tenant when the resource can't be deduced from the token:

```text
/api/v1/app/tenants/{tenantId}/funnels/leads/{leadId}
/api/v1/admin/tenants/{tenantId}/users
/api/v1/admin/tenants/{tenantId}/billing/invoices
```

Frontend:

```text
/app/tenants/{tenantId}/funnels/leads/{leadId}
/admin/tenants/{tenantId}/users
/admin/tenants/{tenantId}/billing/invoices
```

If the app allows switching organizations, keep the active tenant in the
session and use `{tenantId}` only where it is needed for clarity, auditing or
cross-administration.

## CRUD by role

| Operation | End user | Admin |
|---|---|---|
| List | `GET /api/v1/app/funnels/leads` | `GET /api/v1/admin/tenants/{tenantId}/funnels/leads` |
| Detail | `GET /api/v1/app/funnels/leads/{leadId}` | `GET /api/v1/admin/tenants/{tenantId}/funnels/leads/{leadId}` |
| Create | `POST /api/v1/app/funnels/leads` | `POST /api/v1/admin/tenants/{tenantId}/funnels/leads` |
| Update | `PATCH /api/v1/app/funnels/leads/{leadId}` | `PATCH /api/v1/admin/tenants/{tenantId}/funnels/leads/{leadId}` |
| Delete | `DELETE /api/v1/app/funnels/leads/{leadId}` | `DELETE /api/v1/admin/tenants/{tenantId}/funnels/leads/{leadId}` |
| Suspend user | — | `POST /api/v1/admin/tenants/{tenantId}/users/{userId}/suspend` |

## Granular permissions

Permissions by **resource and action**, never by URL:

```text
funnels.leads.read
funnels.leads.create
funnels.leads.update
funnels.leads.delete
campaigns.launch
tenant.users.manage
billing.invoices.read
platform.tenants.manage
```

Forbidden response shape:

```json
{
  "error": {
    "code": "FORBIDDEN",
    "message": "You do not have permission to manage users in this tenant.",
    "requiredPermission": "tenant.users.manage"
  }
}
```

## Frontend: guards and layouts

```text
/app/(authenticated)/funnels/leads/[leadId]/page
/admin/(admin)/tenants/[tenantId]/users/page
/partner/(partner)/campaigns/[campaignId]/page
```

- `/app`: end-user navigation layout.
- `/admin`: separate layout — admin sidebar + admin role guard.
- `/partner`: partner layout.
- Guards redirect to `/login`, `/403` or the allowed dashboard; hiding a
  button is never the actual protection.

## Concrete example for Heimdall

```text
Frontend
/app/projects/{projectId}
/app/projects/{projectId}/funnels/{funnelId}
/app/funnels/leads/{leadId}
/admin/tenants/{tenantId}/users
/admin/tenants/{tenantId}/usage
/admin/tenants/{tenantId}/billing

API
GET  /api/v1/app/projects/{projectId}/runs            # Apify extraction runs
POST /api/v1/app/projects/{projectId}/launch          # launch the campaign
GET  /api/v1/app/funnels/leads/{leadId}
GET  /api/v1/admin/tenants/{tenantId}/users
POST /api/v1/admin/tenants/{tenantId}/users/{userId}/suspend
```

## Checklist

- [ ] `/app`, `/admin`, `/partner`, … are experience namespaces.
- [ ] No generic `/user`; use `/app`.
- [ ] `{userType}` never replaces authentication or RBAC.
- [ ] Validate tenant, role, permission and resource ownership in the backend.
- [ ] Include `{tenantId}` on admin and multi-tenant routes.
- [ ] plural resources, IDs in path, filters in query.
- [ ] Permissions of the form `module.resource.action` — URLs are never the
      source of truth.
- [ ] Separate layouts and guards per user type in the frontend.
