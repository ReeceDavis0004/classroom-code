use sqlx::PgPool;

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn schema_check_detects_missing_migration(pool: PgPool) {
    assert!(classroom_core::schema_is_current(&pool).await.unwrap());
    sqlx::query(
        "DELETE FROM _sqlx_migrations WHERE version = (SELECT max(version) FROM _sqlx_migrations)",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(!classroom_core::schema_is_current(&pool).await.unwrap());
}
