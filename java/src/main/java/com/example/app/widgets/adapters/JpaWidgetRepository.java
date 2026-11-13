package com.example.app.widgets.adapters;

import com.example.app.shared.TenantId;
import com.example.app.widgets.application.WidgetRepository;
import com.example.app.widgets.domain.Widget;
import com.example.app.widgets.domain.WidgetId;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.persistence.EntityManager;
import jakarta.persistence.PersistenceContext;
import java.util.Optional;

@ApplicationScoped
public final class JpaWidgetRepository implements WidgetRepository {
    @PersistenceContext
    private EntityManager entityManager;

    @Override
    public Optional<Widget> find(TenantId tenant, WidgetId id) {
        return entityManager
                .createQuery("select w from WidgetEntity w where w.id = :id and w.tenantId = :tenant",
                        WidgetEntity.class)
                .setParameter("id", id.value())
                .setParameter("tenant", tenant.value())
                .getResultStream()
                .findFirst()
                .map(WidgetEntity::toDomain);
    }

    @Override
    public void save(TenantId tenant, Widget widget) {
        if (!tenant.equals(widget.tenantId())) {
            throw new IllegalArgumentException("tenant mismatch");
        }
        entityManager.merge(WidgetEntity.from(widget));
    }
}
