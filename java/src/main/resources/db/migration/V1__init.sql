-- The schema is the source of truth: Flyway applies it, Hibernate validates it.
create table widgets (
    id         text primary key,
    tenant_id  text not null,
    name       text not null,
    status     text not null,
    version    bigint not null default 0,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index widgets_tenant_idx on widgets (tenant_id, id);

alter table widgets enable row level security;
create policy widgets_tenant_isolation on widgets
    using (tenant_id = current_setting('app.tenant_id', true));

create table outbox (
    id              text primary key,
    tenant_id       text not null,
    type            text not null,
    payload         text not null,
    idempotency_key text not null unique,
    created_at      timestamptz not null default now(),
    sent_at         timestamptz
);

create index outbox_due_idx on outbox (sent_at, created_at);
