use sqlx::PgPool;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

pub async fn schema_is_current(pool: &PgPool) -> sqlx::Result<bool> {
    let applied: Option<i64> =
        match sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations WHERE success")
            .fetch_one(pool)
            .await
        {
            Ok(version) => version,
            Err(e) if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("42P01") => {
                None
            }
            Err(e) => return Err(e),
        };
    let embedded = MIGRATOR.iter().map(|m| m.version).max();
    Ok(applied >= embedded)
}
