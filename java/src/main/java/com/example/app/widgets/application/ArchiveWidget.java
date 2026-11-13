package com.example.app.widgets.application;

import com.example.app.shared.Result;
import com.example.app.shared.TenantId;
import com.example.app.widgets.domain.Widget;
import com.example.app.widgets.domain.WidgetId;

public interface ArchiveWidget {
    Result<Widget> archive(TenantId tenant, WidgetId id, long expectedVersion, String idempotencyKey);
}
