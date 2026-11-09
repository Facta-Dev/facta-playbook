//! Postgres adapter for `WidgetRepository`. The ONLY place SQL lives.
//!
//! All queries are runtime `sqlx::query*` calls — no compile-time macros, so
//! builds need no `DATABASE_URL` or `.sqlx` cache (the template ships
//! neither; a fresh clone must build offline).
use crate::domain::WidgetRepository;
use async_trait::async_trait;
use <app>_core::{Error, Result, Widget, WidgetId, WidgetKind};
use sqlx::PgPool;

/// Concrete adapter. Cheap to clone — the pool is an `Arc` inside.
#[derive(Debug, Clone)]
pub struct PgWidgetRepository {
    pool: PgPool,
}

impl PgWidgetRepository {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Map the foreign error into the domain error at the boundary.
fn map(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound,
        other => Error::Internal(other.to_string()),
    }
}

#[async_trait]
impl WidgetRepository for PgWidgetRepository {
    async fn find(&self, id: WidgetId) -> Result<Option<Widget>> {
        let row =
            sqlx::query_as::<_, WidgetRow>("SELECT id, name, kind FROM widgets WHERE id = $1")
                .bind(id.0)
                .fetch_optional(&self.pool)
                .await
                .map_err(map)?;
        Ok(row.map(Into::into))
    }

    async fn list(&self) -> Result<Vec<Widget>> {
        let rows =
            sqlx::query_as::<_, WidgetRow>("SELECT id, name, kind FROM widgets ORDER BY name ASC")
                .fetch_all(&self.pool)
                .await
                .map_err(map)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn upsert(&self, widget: Widget) -> Result<Widget> {
        let row = sqlx::query_as::<_, WidgetRow>(
            "INSERT INTO widgets (id, name, kind) VALUES ($1, $2, $3)
             ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, kind = EXCLUDED.kind
             RETURNING id, name, kind",
        )
        .bind(widget.id.0)
        .bind(&widget.name)
        .bind(widget.kind.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(map)?;
        Ok(row.into())
    }

    async fn delete(&self, id: WidgetId) -> Result<bool> {
        let result = sqlx::query("DELETE FROM widgets WHERE id = $1")
            .bind(id.0)
            .execute(&self.pool)
            .await
            .map_err(map)?;
        Ok(result.rows_affected() > 0)
    }

    async fn update(&self, id: WidgetId, name: &str, kind: WidgetKind) -> Result<Widget> {
        let row = sqlx::query_as::<_, WidgetRow>(
            "UPDATE widgets SET name = $2, kind = $3 WHERE id = $1 RETURNING id, name, kind",
        )
        .bind(id.0)
        .bind(name)
        .bind(kind.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map)?;
        row.map(Into::into).ok_or(Error::NotFound)
    }
}

// Anti-corruption seam: the row struct stays local to the adapter so the
// domain `Widget` never learns about sqlx column layout. `FromRow` maps the
// runtime query row into this struct.
#[derive(Debug, sqlx::FromRow)]
struct WidgetRow {
    id: uuid::Uuid,
    name: String,
    kind: String,
}

impl From<WidgetRow> for Widget {
    fn from(r: WidgetRow) -> Self {
        Self {
            id: <app>_core::WidgetId(r.id),
            name: r.name,
            kind: match r.kind.as_str() {
                "cog" => <app>_core::WidgetKind::Cog,
                "flange" => <app>_core::WidgetKind::Flange,
                _ => <app>_core::WidgetKind::Sprocket,
            },
        }
    }
}
