package com.example.app.widgets.application;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.example.app.shared.Clock;
import com.example.app.shared.Result;
import com.example.app.shared.TenantId;
import com.example.app.widgets.domain.Widget;
import com.example.app.widgets.domain.WidgetId;
import com.example.app.widgets.domain.WidgetStatus;
import java.time.Instant;
import java.util.HashMap;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;

class ArchiveWidgetServiceTest {
    private static final TenantId TENANT = new TenantId("t1");
    private static final WidgetId WIDGET = new WidgetId("w1");

    private final Map<WidgetId, Widget> store = new HashMap<>();
    private final WidgetRepository repository = new WidgetRepository() {
        @Override
        public Optional<Widget> find(TenantId tenant, WidgetId id) {
            return Optional.ofNullable(store.get(id));
        }

        @Override
        public void save(TenantId tenant, Widget widget) {
            store.put(widget.id(), widget);
        }
    };
    private final Outbox outbox = (tenant, type, payload, key) -> {
    };
    private final Clock clock = () -> Instant.parse("2026-01-01T00:00:00Z");

    private ArchiveWidgetService service() {
        return new ArchiveWidgetService(repository, outbox, clock);
    }

    @Test
    void archivesAReadyWidget() {
        store.put(WIDGET, new Widget(WIDGET, TENANT, "First", WidgetStatus.READY, 0));

        Result<Widget> result = service().archive(TENANT, WIDGET, 0, "idem-1");

        assertTrue(result instanceof Result.Success<?>);
        Widget archived = (Widget) ((Result.Success<?>) result).value();
        assertEquals(WidgetStatus.ARCHIVED, archived.status());
    }

    @Test
    void rejectsAStaleVersion() {
        store.put(WIDGET, new Widget(WIDGET, TENANT, "First", WidgetStatus.READY, 2));

        Result<Widget> result = service().archive(TENANT, WIDGET, 0, "idem-2");

        assertTrue(result instanceof Result.Failure<?>);
        assertEquals("CONFLICT", ((Result.Failure<?>) result).error().code());
    }

    @Test
    void returnsNotFoundForAnUnknownWidget() {
        Result<Widget> result = service().archive(TENANT, WIDGET, 0, "idem-3");

        assertTrue(result instanceof Result.Failure<?>);
        assertEquals("NOT_FOUND", ((Result.Failure<?>) result).error().code());
    }
}
