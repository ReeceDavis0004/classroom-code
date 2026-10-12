use sqlx::PgPool;

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn worker_role_cannot_read_secrets(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    sqlx::query("SET ROLE app_worker")
        .execute(&mut *conn)
        .await
        .unwrap();

    sqlx::query("SELECT question_id, stdin, expected_stdout FROM test_cases")
        .execute(&mut *conn)
        .await
        .unwrap();

    for sql in [
        "SELECT email FROM professors",
        "SELECT token_hash FROM participants",
        "SELECT share_token FROM rooms",
        "SELECT id_hash FROM professor_sessions",
    ] {
        let err = sqlx::query(sql).execute(&mut *conn).await.unwrap_err();
        let code = err.as_database_error().and_then(|e| e.code());
        assert_eq!(code.as_deref(), Some("42501"), "{sql}");
    }
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn api_role_can_write_app_tables(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    sqlx::query("SET ROLE app_api")
        .execute(&mut *conn)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO professors (id, email, password_hash, display_name)
         VALUES (gen_random_uuid(), 'prof_1@example.com', 'x', 'prof_1')",
    )
    .execute(&mut *conn)
    .await
    .unwrap();
    let err = sqlx::query("CREATE TABLE t (x int)")
        .execute(&mut *conn)
        .await
        .unwrap_err();
    let code = err.as_database_error().and_then(|e| e.code());
    assert_eq!(code.as_deref(), Some("42501"));
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn deleting_a_game_room_cascades(pool: PgPool) {
    sqlx::query(
        "WITH p AS (INSERT INTO professors (id, email, password_hash, display_name)
                    VALUES (gen_random_uuid(), 'prof_1@example.com', 'x', 'prof_1') RETURNING id),
              r AS (INSERT INTO rooms (id, professor_id, title, mode, share_token, game_state)
                    SELECT '00000000-0000-7000-8000-000000000001', id, 'Room', 'game', 'tok', 'live'
                    FROM p RETURNING id)
         INSERT INTO questions (id, room_id, ordinal, title, languages, time_limit_ms, memory_limit_mb)
         SELECT '00000000-0000-7000-8000-000000000002', id, 1, 'Q', '{c}', 1000, 256 FROM r",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE rooms SET current_question_id = '00000000-0000-7000-8000-000000000002'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM rooms")
        .execute(&pool)
        .await
        .unwrap();
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM questions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
}
