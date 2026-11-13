package com.example.app.widgets.application;

import com.example.app.shared.AppError;
import com.example.app.shared.Clock;
import com.example.app.shared.Result;
import com.example.app.shared.TenantId;
import com.example.app.widgets.domain.Widget;
import com.example.app.widgets.domain.WidgetId;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import jakarta.transaction.Transactional;

@ApplicationScoped
public final class ArchiveWidgetService implements ArchiveWidget {
    private final WidgetRepository widgets;
    private final Outbox outbox;
    private final Clock clock;

    @Inject
    public ArchiveWidgetService(WidgetRepository widgets, Outbox outbox, Clock clock) {
        this.widgets = widgets;
        this.outbox = outbox;
        this.clock = clock;
    }

    @Override
    @Transactional
    public Result<Widget> archive(TenantId tenant, WidgetId id, long expectedVersion, String idempotencyKey) {
        return widgets.find(tenant, id)
                .map(widget -> {
                    if (widget.version() != expectedVersion) {
                        return Result.<Widget>failure(AppError.conflict());
                    }
                    Widget archived = widget.archive();
                    widgets.save(tenant, archived);
                    outbox.append(tenant, "widget.archived", archived.id().value(), idempotencyKey);
                    return Result.success(archived);
                })
                .orElseGet(() -> Result.failure(AppError.notFound()));
    }
}
