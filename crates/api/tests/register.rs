mod common;

use axum::http::StatusCode;
use common::{PASSWORD, TestApp};
use serde_json::json;
use sqlx::PgPool;

const REGISTER: &str = "/api/v1/pub/auth/register";

fn body(code: &str, email: &str) -> serde_json::Value {
    json!({"invite_code": code, "email": email, "display_name": "prof_1", "password": PASSWORD})
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn valid_invite_creates_unverified_professor(pool: PgPool) {
    let app = TestApp::new(pool);
    let r = app.register("prof_1@example.com").await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert_eq!(r.json["status"], "unverified");
    assert_eq!(r.json["email"], "prof_1@example.com");

    let mails = app.mails();
    assert_eq!(mails.len(), 1);
    assert_eq!(mails[0].to, "prof_1@example.com");
    assert!(
        mails[0]
            .body
            .contains("https://class.example.com/verify?token=")
    );

    let hash: String = sqlx::query_scalar("SELECT password_hash FROM professors")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert!(
        hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
        "{hash}"
    );
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn invalid_invite_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let r = app
        .post(REGISTER, body("not-a-real-code", "prof_1@example.com"))
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json["code"], "validation_failed");
    assert_eq!(r.json["errors"][0]["field"], "invite_code");
    assert!(app.mails().is_empty());
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn expired_invite_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let code = app.invite().await;
    sqlx::query("UPDATE professor_invites SET expires_at = now() - interval '1 second'")
        .execute(&app.pool)
        .await
        .unwrap();
    let r = app.post(REGISTER, body(&code, "prof_1@example.com")).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json["errors"][0]["field"], "invite_code");
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn reused_invite_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let code = app.invite().await;
    let first = app.post(REGISTER, body(&code, "prof_1@example.com")).await;
    assert_eq!(first.status, StatusCode::CREATED);
    let second = app.post(REGISTER, body(&code, "prof_2@example.com")).await;
    assert_eq!(second.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(second.json["errors"][0]["field"], "invite_code");
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn taken_email_does_not_consume_invite(pool: PgPool) {
    let app = TestApp::new(pool);
    app.register("prof_1@example.com").await;
    let code = app.invite().await;
    let taken = app.post(REGISTER, body(&code, "PROF_1@example.com")).await;
    assert_eq!(taken.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(taken.json["errors"][0]["field"], "email");
    let ok = app.post(REGISTER, body(&code, "prof_2@example.com")).await;
    assert_eq!(ok.status, StatusCode::CREATED);
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn short_password_and_bad_email_are_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let code = app.invite().await;
    let r = app
        .post(
            REGISTER,
            json!({"invite_code": code, "email": "nope", "display_name": "prof_1", "password": "short"}),
        )
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    let fields: Vec<_> = r.json["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["email", "password"]);
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn verify_activates_once(pool: PgPool) {
    let app = TestApp::new(pool);
    app.register("prof_1@example.com").await;
    let token = app.last_token();

    let refused = app.login("prof_1@example.com", PASSWORD).await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED);

    let r = app
        .post("/api/v1/pub/auth/verify", json!({"token": token}))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let again = app
        .post("/api/v1/pub/auth/verify", json!({"token": token}))
        .await;
    assert_eq!(again.status, StatusCode::UNPROCESSABLE_ENTITY);

    let ok = app.login("prof_1@example.com", PASSWORD).await;
    assert_eq!(ok.status, StatusCode::OK);
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn expired_verify_token_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    app.register("prof_1@example.com").await;
    let token = app.last_token();
    sqlx::query("UPDATE email_tokens SET expires_at = now() - interval '1 second'")
        .execute(&app.pool)
        .await
        .unwrap();
    let r = app
        .post("/api/v1/pub/auth/verify", json!({"token": token}))
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
}
