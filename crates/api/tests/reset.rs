mod common;

use axum::http::StatusCode;
use common::{TestApp, authed};
use serde_json::json;
use sqlx::PgPool;

const NEW_PASSWORD: &str = "another long passphrase";

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn unknown_email_gets_202_and_no_mail(pool: PgPool) {
    let app = TestApp::new(pool);
    let r = app
        .post(
            "/api/v1/pub/auth/reset-request",
            json!({"email": "nobody@example.com"}),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED);
    assert!(app.mails().is_empty());
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn reset_sets_password_once_and_revokes_sessions(pool: PgPool) {
    let app = TestApp::new(pool);
    let auth = app.professor("prof_1@example.com").await;

    let r = app
        .post(
            "/api/v1/pub/auth/reset-request",
            json!({"email": "prof_1@example.com"}),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED);
    let mail = app.mails().pop().unwrap();
    assert_eq!(mail.to, "prof_1@example.com");
    assert!(mail.body.contains("https://class.example.com/reset?token="));
    let token = app.last_token();

    let reset = json!({"token": token, "password": NEW_PASSWORD});
    let ok = app.post("/api/v1/pub/auth/reset", reset.clone()).await;
    assert_eq!(ok.status, StatusCode::NO_CONTENT);
    let again = app.post("/api/v1/pub/auth/reset", reset).await;
    assert_eq!(again.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(again.json["errors"][0]["field"], "token");

    let old_session = app
        .send(authed("GET", "/api/v1/p/session", &auth), None)
        .await;
    assert_eq!(old_session.status, StatusCode::UNAUTHORIZED);
    let login = app.login("prof_1@example.com", NEW_PASSWORD).await;
    assert_eq!(login.status, StatusCode::OK);
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn expired_reset_token_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    app.register_verified("prof_1@example.com").await;
    app.post(
        "/api/v1/pub/auth/reset-request",
        json!({"email": "prof_1@example.com"}),
    )
    .await;
    let token = app.last_token();
    sqlx::query(
        "UPDATE email_tokens SET expires_at = now() - interval '1 second' WHERE purpose = 'reset'",
    )
    .execute(&app.pool)
    .await
    .unwrap();
    let r = app
        .post(
            "/api/v1/pub/auth/reset",
            json!({"token": token, "password": NEW_PASSWORD}),
        )
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
}
