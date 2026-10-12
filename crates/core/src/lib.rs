use sqlx::PgPool;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

pub async fn schema_is_current(pool: &PgPool) -> sqlx::Result<bool> {
    let applied: Option<i64> =
        sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations WHERE success")
            .fetch_one(pool)
            .await?;
    let embedded = MIGRATOR.iter().map(|m| m.version).max();
    Ok(applied >= embedded)
}
