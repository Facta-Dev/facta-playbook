//! In-memory test double for `WidgetRepository`. Lives in `ports` so every
//! downstream crate reuses it. Gate it: `#[cfg(any(test, feature = "mock"))]`.
use std::collections::HashMap;
use std::sync::Mutex;

use crate::domain::WidgetRepository;
use async_trait::async_trait;
use <app>_core::{Error, Result, Widget, WidgetId, WidgetKind};

#[derive(Debug, Default)]
pub struct MockWidgetRepository {
    rows: Mutex<HashMap<WidgetId, Widget>>,
}

#[async_trait]
impl WidgetRepository for MockWidgetRepository {
    async fn find(&self, id: WidgetId) -> Result<Option<Widget>> {
        Ok(self.rows.lock().unwrap().get(&id).cloned())
    }
    async fn list(&self) -> Result<Vec<Widget>> {
        Ok(self.rows.lock().unwrap().values().cloned().collect())
    }
    async fn upsert(&self, widget: Widget) -> Result<Widget> {
        self.rows.lock().unwrap().insert(widget.id, widget.clone());
        Ok(widget)
    }
    async fn update(&self, id: WidgetId, name: &str, kind: WidgetKind) -> Result<Widget> {
        let exists = self.rows.lock().unwrap().contains_key(&id);
        if !exists {
            return Err(Error::NotFound);
        }
        let widget = Widget {
            id,
            name: name.to_owned(),
            kind,
        };
        self.rows.lock().unwrap().insert(id, widget.clone());
        Ok(widget)
    }
    async fn delete(&self, id: WidgetId) -> Result<bool> {
        Ok(self.rows.lock().unwrap().remove(&id).is_some())
    }
}
