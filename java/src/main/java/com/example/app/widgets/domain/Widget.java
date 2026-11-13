package com.example.app.widgets.domain;

import com.example.app.shared.TenantId;
import java.util.Objects;

public record Widget(WidgetId id, TenantId tenantId, String name, WidgetStatus status, long version) {
    public Widget {
        Objects.requireNonNull(id, "id");
        Objects.requireNonNull(tenantId, "tenantId");
        Objects.requireNonNull(name, "name");
        Objects.requireNonNull(status, "status");
    }

    public Widget archive() {
        if (status == WidgetStatus.ARCHIVED) {
            throw new IllegalStateException("widget is already archived");
        }
        return new Widget(id, tenantId, name, WidgetStatus.ARCHIVED, version + 1);
    }
}
