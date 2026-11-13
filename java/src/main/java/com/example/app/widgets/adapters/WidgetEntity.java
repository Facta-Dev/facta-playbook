package com.example.app.widgets.adapters;

import com.example.app.shared.TenantId;
import com.example.app.widgets.domain.Widget;
import com.example.app.widgets.domain.WidgetId;
import com.example.app.widgets.domain.WidgetStatus;
import jakarta.persistence.Column;
import jakarta.persistence.Entity;
import jakarta.persistence.Id;
import jakarta.persistence.Table;
import jakarta.persistence.Version;

@Entity
@Table(name = "widgets")
public class WidgetEntity {
    @Id
    private String id;

    @Column(name = "tenant_id", nullable = false)
    private String tenantId;

    @Column(nullable = false)
    private String name;

    @Column(nullable = false)
    private String status;

    @Version
    private long version;

    protected WidgetEntity() {
    }

    public static WidgetEntity from(Widget widget) {
        WidgetEntity entity = new WidgetEntity();
        entity.id = widget.id().value();
        entity.tenantId = widget.tenantId().value();
        entity.name = widget.name();
        entity.status = widget.status().name();
        entity.version = widget.version();
        return entity;
    }

    public Widget toDomain() {
        return new Widget(
                new WidgetId(id),
                new TenantId(tenantId),
                name,
                WidgetStatus.valueOf(status),
                version);
    }
}
