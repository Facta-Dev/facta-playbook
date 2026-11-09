//! Postgres adapter for `UserRepository`. The ONLY place identity SQL for
//! the `users` table lives. Runtime `sqlx::query*` calls — no compile-time
//! macros, so builds need no `DATABASE_URL` or `.sqlx` cache.
//!
//! SQL semantics mirror the Go identity features 1:1 (authsignup /
//! authsession / authpassword / authoauth / adminusers posts.go).
use crate::domain::{UpdateMeInput, UserPatch, UserRepository};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use <app>_core::{Error, Result, Role, User, UserId, UserType};
use sqlx::PgPool;

/// The users projection this adapter maps onto [`<app>_core::User`].
const USER_COLUMNS: &str = "id, email, first_name, last_name, role, user_type, \
     language, is_active, is_blocked, is_verified, retry_count, password, \
     profile_picture, google_id, github_id, last_login, created_at, updated_at";

/// Concrete adapter. Cheap to clone — the pool is an `Arc` inside.
#[derive(Debug, Clone)]
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
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

/// DB role strings ('ADMIN'|'MODERATOR'|'USER'|'GUEST', 0001 CHECK parity)
/// onto the provisional core mapping.
fn role_from_db(s: &str) -> Role {
    match s {
        "ADMIN" => Role::Admin,
        "MODERATOR" => Role::Editor,
        _ => Role::Viewer,
    }
}

/// Role onto the canonical DB string (Go parity: the columns store the Go
/// strings; see `<app>_core::Role::as_db_str` for the pinned twin).
const fn role_db(role: Role) -> &'static str {
    match role {
        Role::Admin | Role::Owner => "ADMIN",
        Role::Editor => "MODERATOR",
        Role::Viewer => "USER",
    }
}

/// `user_type` DB strings ('ADMIN'|'COMPANY'|'USER', 0001 CHECK parity).
fn user_type_from_db(s: &str) -> UserType {
    match s {
        "ADMIN" => UserType::Admin,
        "USER" => UserType::User,
        _ => UserType::Company,
    }
}

/// `user_type` onto the canonical DB string (0001 CHECK parity).
const fn user_type_db(u: UserType) -> &'static str {
    match u {
        UserType::Admin => "ADMIN",
        UserType::User => "USER",
        UserType::Company => "COMPANY",
    }
}
/// Anti-corruption seam: the row struct stays local to the adapter (pinned
/// `XRow` pattern).
#[derive(Debug, sqlx::FromRow)]
struct UserRow {
    id: uuid::Uuid,
    email: String,
    first_name: String,
    last_name: String,
    role: String,
    user_type: String,
    language: String,
    is_active: bool,
    is_blocked: bool,
    is_verified: bool,
    retry_count: i32,
    password: String,
    profile_picture: Option<String>,
    google_id: Option<String>,
    github_id: Option<String>,
    last_login: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        Self {
            id: UserId(r.id),
            email: r.email,
            first_name: r.first_name,
            last_name: r.last_name,
            role: role_from_db(&r.role),
            user_type: user_type_from_db(&r.user_type),
            is_active: r.is_active,
            is_blocked: r.is_blocked,
            is_verified: r.is_verified,
            retry_count: r.retry_count,
            password: r.password,
            language: r.language,
            profile_picture: r.profile_picture,
            google_id: r.google_id,
            github_id: r.github_id,
            last_login: r.last_login,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// One-flag RETURNING row (`SessionAfterFailedLogin`).
#[derive(Debug, sqlx::FromRow)]
struct FlagRow {
    is_blocked: bool,
}

fn fetch_full_sql() -> String {
    format!("SELECT {USER_COLUMNS} FROM users")
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        self.get_user_by_column("email", email).await
    }

    async fn get_user_by_id(&self, user_id: UserId) -> Result<Option<User>> {
        let row = sqlx::query_as::<_, UserRow>(&format!("{} WHERE id = $1", fetch_full_sql()))
            .bind(user_id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(map)?;
        Ok(row.map(Into::into))
    }

    async fn get_user_by_google_id(&self, google_id: &str) -> Result<Option<User>> {
        self.get_user_by_column("google_id", google_id).await
    }

    async fn get_user_by_github_id(&self, github_id: &str) -> Result<Option<User>> {
        self.get_user_by_column("github_id", github_id).await
    }

    async fn create_user(&self, user: &User) -> Result<()> {
        sqlx::query(
            "INSERT INTO users (id, email, first_name, last_name, role, user_type, \
             is_active, is_blocked, is_verified, retry_count, password, language, \
             profile_picture, google_id, github_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(user.id.0)
        .bind(&user.email)
        .bind(&user.first_name)
        .bind(&user.last_name)
        .bind(role_db(user.role))
        .bind(user_type_db(user.user_type))
        .bind(user.is_active)
        .bind(user.is_blocked)
        .bind(user.is_verified)
        .bind(user.retry_count)
        .bind(&user.password)
        .bind(&user.language)
        .bind(&user.profile_picture)
        .bind(&user.google_id)
        .bind(&user.github_id)
        .execute(&self.pool)
        .await
        .map_err(|e| unique_violation(e, "email already in use"))?;
        Ok(())
    }

    async fn update(&self, user_id: UserId, patch: &UserPatch) -> Result<()> {
        sqlx::query(
            "UPDATE users SET \
             is_active = COALESCE($2, is_active), \
             is_blocked = COALESCE($3, is_blocked), \
             role = COALESCE($4, role), updated_at = now() \
             WHERE id = $1",
        )
        .bind(user_id.0)
        .bind(patch.is_active)
        .bind(patch.is_blocked)
        .bind(patch.role.map(role_db))
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn update_me(&self, user_id: UserId, patch: &UpdateMeInput) -> Result<Option<User>> {
        let result = sqlx::query(
            "UPDATE users SET \
             first_name = COALESCE($2, first_name), \
             last_name = COALESCE($3, last_name), \
             phone_number = COALESCE($4, phone_number), \
             profile_picture = COALESCE($5, profile_picture), \
             language = COALESCE($6, language), \
             address = COALESCE($7, address), \
             postal_code = COALESCE($8, postal_code), \
             updated_at = now() \
             WHERE id = $1",
        )
        .bind(user_id.0)
        .bind(&patch.first_name)
        .bind(&patch.last_name)
        .bind(&patch.phone)
        .bind(&patch.picture)
        .bind(&patch.language)
        .bind(&patch.address)
        .bind(&patch.postal)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }
        self.get_user_by_id(user_id).await
    }

    async fn upgrade_to_admin(&self, user_id: UserId) -> Result<()> {
        sqlx::query(
            "UPDATE users SET role = 'ADMIN', user_type = 'ADMIN', updated_at = now() \
             WHERE id = $1",
        )
        .bind(user_id.0)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn after_failed_login(&self, user_id: UserId, lock_at: i32) -> Result<bool> {
        let row = sqlx::query_as::<_, FlagRow>(
            "UPDATE users SET retry_count = retry_count + 1, \
             is_blocked = (retry_count + 1 >= $2), updated_at = now() \
             WHERE id = $1 RETURNING is_blocked",
        )
        .bind(user_id.0)
        .bind(lock_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map)?;
        Ok(row.is_blocked)
    }

    async fn after_successful_login(&self, user_id: UserId) -> Result<()> {
        sqlx::query(
            "UPDATE users SET retry_count = 0, last_login = now(), updated_at = now() \
             WHERE id = $1",
        )
        .bind(user_id.0)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn link_google(&self, user_id: UserId, google_id: &str, picture: &str) -> Result<()> {
        sqlx::query(
            "UPDATE users SET google_id = $2, is_verified = true, \
             profile_picture = COALESCE(NULLIF(profile_picture, ''), $3), \
             updated_at = now() WHERE id = $1",
        )
        .bind(user_id.0)
        .bind(google_id)
        .bind(picture)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn link_github(&self, user_id: UserId, github_id: &str, picture: &str) -> Result<()> {
        sqlx::query(
            "UPDATE users SET github_id = $2, is_verified = true, \
             profile_picture = COALESCE(NULLIF(profile_picture, ''), $3), \
             updated_at = now() WHERE id = $1",
        )
        .bind(user_id.0)
        .bind(github_id)
        .bind(picture)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn github_only(&self, user_id: UserId, github_id: &str) -> Result<()> {
        sqlx::query("UPDATE users SET github_id = $2, updated_at = now() WHERE id = $1")
            .bind(user_id.0)
            .bind(github_id)
            .execute(&self.pool)
            .await
            .map_err(map)?;
        Ok(())
    }

    async fn save_github_login(&self, user_id: UserId, email: &str, picture: &str) -> Result<()> {
        sqlx::query(
            "UPDATE users SET email = $2, is_verified = true, \
             profile_picture = COALESCE(NULLIF(profile_picture, ''), $3), \
             updated_at = now() WHERE id = $1",
        )
        .bind(user_id.0)
        .bind(email)
        .bind(picture)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn list_users(&self, limit: i64, offset: i64) -> Result<Vec<User>> {
        let rows = sqlx::query_as::<_, UserRow>(&format!(
            "{} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            fetch_full_sql()
        ))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn count_users(&self) -> Result<i64> {
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(map)?;
        Ok(n)
    }
}

impl PgUserRepository {
    /// Shared exact-match read over one unique column.
    async fn get_user_by_column(&self, column: &'static str, value: &str) -> Result<Option<User>> {
        let row =
            sqlx::query_as::<_, UserRow>(&format!("{} WHERE {column} = $1", fetch_full_sql()))
                .bind(value)
                .fetch_optional(&self.pool)
                .await
                .map_err(map)?;
        Ok(row.map(Into::into))
    }
}

/// Go parity: a lost signup race (unique violation 23505 on email /
/// provider-id columns) answers as the `EMAIL_TAKEN` conflict, not a 500.
fn unique_violation(e: sqlx::Error, detail: &str) -> Error {
    if let sqlx::Error::Database(db) = &e
        && db.code().as_deref() == Some("23505")
    {
        return Error::Conflict(detail.to_owned());
    }
    map(e)
}
