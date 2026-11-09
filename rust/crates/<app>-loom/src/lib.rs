//! Orchestration crate: the realtime hub (rooms + event fan-out + WS pump)
//! and the background worker loops (outbox drain + janitor, mirror of go
//! `cmd/worker`). Same pattern as a leaf slice, wider dependency list; the
//! worker binary lives here too as `<app>-worker`.
use <app>_core::Result;
use <app>_widgets::WidgetService;

pub mod hub;
pub mod workers;

pub use hub::{CancelFn, Event, EventBus, Hub, frame, room_user};

/// The orchestrator. Constructed by the composition root with pre-wired
/// services.
#[derive(Debug, Clone)]
pub struct Loom {
    widgets: WidgetService,
}

impl Loom {
    pub const fn new(widgets: WidgetService) -> Self {
        Self { widgets }
    }

    /// ponytail: a real orchestration fans out across several slices. Here we
    /// just delegate to `widgets.list()` to show the shape — the service lives
    /// in `<app>-widgets`, so we call it directly instead of adding an orphan
    /// impl on a foreign type (Rust's orphan rule forbids that).
    pub async fn run(&self) -> Result<()> {
        let _list = self.widgets.list().await?;
        Ok(())
    }
}
