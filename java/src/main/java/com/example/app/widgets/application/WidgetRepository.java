package com.example.app.widgets.application;

import com.example.app.shared.TenantId;
import com.example.app.widgets.domain.Widget;
import com.example.app.widgets.domain.WidgetId;
import java.util.Optional;

public interface WidgetRepository {
    Optional<Widget> find(TenantId tenant, WidgetId id);

    void save(TenantId tenant, Widget widget);
}
