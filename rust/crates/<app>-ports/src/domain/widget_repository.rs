//! Repository port for the widget `<scope>`.
use async_trait::async_trait;
use <app>_core::{Result, Widget, WidgetId, WidgetKind};

/// CRUD contract over widget storage. No I/O assumptions leak through this
/// signature — an implementation could be Postgres, an HTTP service, or a
/// `HashMap`. Consumers depend on THIS, never on the adapter.
#[async_trait]
pub trait WidgetRepository: Send + Sync + 'static {
    async fn find(&self, id: WidgetId) -> Result<Option<Widget>>;
    async fn list(&self) -> Result<Vec<Widget>>;
    async fn upsert(&self, widget: Widget) -> Result<Widget>;
    /// Patch an existing widget's mutable fields. Returns [`Error::NotFound`]
    /// when no row carries `id` (update is NOT an upsert — it must not create).
    async fn update(&self, id: WidgetId, name: &str, kind: WidgetKind) -> Result<Widget>;
    async fn delete(&self, id: WidgetId) -> Result<bool>;
}
