mod common;

use axum::http::StatusCode;
use common::{PASSWORD, TestApp};
use sqlx::PgPool;

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn bad_credentials_get_one_error(pool: PgPool) {
    let app = TestApp::new(pool);
    app.register_verified("prof_1@example.com").await;
    app.register("prof_2@example.com").await;

    let wrong = app.login("prof_1@example.com", "wrong password 123").await;
    let unknown = app.login("nobody@example.com", PASSWORD).await;
    let unverified = app.login("prof_2@example.com", PASSWORD).await;
    for r in [&wrong, &unknown, &unverified] {
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);
        assert_eq!(r.json["code"], "unauthenticated");
        assert!(r.headers.get("set-cookie").is_none());
    }
    assert_eq!(wrong.json, unknown.json);
    assert_eq!(wrong.json, unverified.json);
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn login_sets_host_cookie_and_stores_only_its_hash(pool: PgPool) {
    let app = TestApp::new(pool);
    app.register_verified("prof_1@example.com").await;
    let r = app.login("Prof_1@Example.com", PASSWORD).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["professor"]["email"], "prof_1@example.com");

    let cookie = r.headers["set-cookie"].to_str().unwrap();
    for attr in ["Path=/", "Secure", "HttpOnly", "SameSite=Lax"] {
        assert!(cookie.contains(attr), "{cookie}");
    }
    let value = cookie
        .split(';')
        .next()
        .unwrap()
        .strip_prefix("__Host-ps=")
        .unwrap();
    assert_eq!(value.len(), 43);

    let stored: Vec<u8> = sqlx::query_scalar("SELECT id_hash FROM professor_sessions")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(stored, api::token::sha256(value));
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn each_login_gets_a_new_session(pool: PgPool) {
    let app = TestApp::new(pool);
    app.register_verified("prof_1@example.com").await;
    let a = app.login_ok("prof_1@example.com").await;
    let b = app.login_ok("prof_1@example.com").await;
    assert_ne!(a.cookie, b.cookie);
    assert_ne!(a.csrf, b.csrf);
}
