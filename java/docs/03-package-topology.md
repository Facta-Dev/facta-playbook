# Package topology

```text
com.example.app.shared          Result, AppError, TenantId, Clock
com.example.app.health          HealthResource
com.example.app.widgets.domain  Widget, WidgetId, WidgetStatus
com.example.app.widgets.application ArchiveWidget, ports, service
com.example.app.widgets.adapters   JPA entities/repositories, outbox
com.example.app.widgets.presentation WidgetResource, DTO, error envelope
```

The package name stays `com.example.app` in the template; rename it once for a real product.
