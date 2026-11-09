//! Widgets use-case: business rules over widget storage.
//!
//! ponytail: this is the example `<scope>` slice. Replace `widgets` with your
//! own slice, or add more `<app>-<scope>` crates following the same pattern.
use std::sync::Arc;

use <app>_core::{Error, Result, Widget, WidgetId, WidgetKind};
use <app>_ports::WidgetRepository;

/// The slice's service. Constructed by the composition root with a concrete
/// repo; tested with a mock. Cheap to clone.
#[derive(Clone)]
pub struct WidgetService {
    repo: Arc<dyn WidgetRepository>,
}

impl std::fmt::Debug for WidgetService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetService").finish_non_exhaustive()
    }
}

impl WidgetService {
    pub const fn new(repo: Arc<dyn WidgetRepository>) -> Self {
        Self { repo }
    }

    /// A business operation: validate, then persist.
    pub async fn create(&self, name: &str, kind: WidgetKind) -> Result<Widget> {
        if name.trim().is_empty() {
            return Err(Error::Invalid("name must not be empty".into()));
        }
        let widget = Widget {
            id: WidgetId::new(),
            name: name.to_owned(),
            kind,
        };
        self.repo.upsert(widget).await
    }

    pub async fn get(&self, id: WidgetId) -> Result<Widget> {
        self.repo.find(id).await?.ok_or(Error::NotFound)
    }

    /// List all widgets. Thin pass-through — no domain rules to enforce yet.
    pub async fn list(&self) -> Result<Vec<Widget>> {
        self.repo.list().await
    }

    /// Update an existing widget's mutable fields. Validates the name first,
    /// then delegates to the repo's `update` (which returns `NotFound` when
    /// the id does not exist — update is NOT an upsert).
    pub async fn update(&self, id: WidgetId, name: &str, kind: WidgetKind) -> Result<Widget> {
        if name.trim().is_empty() {
            return Err(Error::Invalid("name must not be empty".into()));
        }
        self.repo.update(id, name, kind).await
    }

    /// Delete a widget by id. Returns `true` if a row was removed, `false` if
    /// the id did not exist (idempotent delete).
    pub async fn delete(&self, id: WidgetId) -> Result<bool> {
        self.repo.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use <app>_ports::mock::MockWidgetRepository; // mock comes for free from `ports`

    #[tokio::test]
    async fn create_rejects_blank_name() {
        let svc = WidgetService::new(Arc::new(MockWidgetRepository::default()));
        assert!(svc.create("  ", WidgetKind::Cog).await.is_err());
    }

    #[tokio::test]
    async fn create_then_get_roundtrips() {
        let svc = WidgetService::new(Arc::new(MockWidgetRepository::default()));
        let w = svc.create("cog-1", WidgetKind::Cog).await.unwrap();
        let got = svc.get(w.id).await.unwrap();
        assert_eq!(w.id, got.id);
    }

    #[tokio::test]
    async fn update_rejects_blank_name() {
        let svc = WidgetService::new(Arc::new(MockWidgetRepository::default()));
        let w = svc.create("cog-1", WidgetKind::Cog).await.unwrap();
        assert!(svc.update(w.id, "  ", WidgetKind::Flange).await.is_err());
    }

    #[tokio::test]
    async fn update_returns_not_found_for_missing_id() {
        let svc = WidgetService::new(Arc::new(MockWidgetRepository::default()));
        let missing = WidgetId::new();
        assert!(svc.update(missing, "cog-2", WidgetKind::Cog).await.is_err());
    }

    #[tokio::test]
    async fn update_then_get_roundtrips() {
        let svc = WidgetService::new(Arc::new(MockWidgetRepository::default()));
        let w = svc.create("cog-1", WidgetKind::Cog).await.unwrap();
        let updated = svc
            .update(w.id, "flange-2", WidgetKind::Flange)
            .await
            .unwrap();
        assert_eq!(updated.id, w.id);
        assert_eq!(updated.name, "flange-2");
        assert_eq!(updated.kind, WidgetKind::Flange);
        let got = svc.get(w.id).await.unwrap();
        assert_eq!(got.name, "flange-2");
    }
}
