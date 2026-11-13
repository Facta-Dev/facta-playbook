package com.example.app.widgets.adapters;

import com.example.app.shared.Clock;
import com.example.app.shared.TenantId;
import com.example.app.widgets.application.Outbox;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import jakarta.persistence.EntityManager;
import jakarta.persistence.PersistenceContext;
import java.util.UUID;

@ApplicationScoped
public final class JpaOutbox implements Outbox {
    @PersistenceContext
    private EntityManager entityManager;

    private final Clock clock;

    @Inject
    public JpaOutbox(Clock clock) {
        this.clock = clock;
    }

    @Override
    public void append(TenantId tenant, String type, String payload, String idempotencyKey) {
        entityManager.persist(new OutboxEntity(
                UUID.randomUUID().toString(),
                tenant.value(),
                type,
                payload,
                idempotencyKey,
                clock.now()));
    }
}
