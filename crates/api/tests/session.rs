mod common;

use axum::http::StatusCode;
use common::{Auth, ORIGIN, PASSWORD, TestApp, authed, req};
use serde_json::json;
use sqlx::PgPool;

const NEW_PASSWORD: &str = "another long passphrase";

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn session_returns_professor_and_csrf(pool: PgPool) {
    let app = TestApp::new(pool);
    let auth = app.professor("prof_1@example.com").await;
    let r = app
        .send(
            req("GET", "/api/v1/p/session").header("cookie", &auth.cookie),
            None,
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["professor"]["email"], "prof_1@example.com");
    assert_eq!(r.json["csrf_token"], auth.csrf.as_str());

    let anonymous = app.send(req("GET", "/api/v1/p/session"), None).await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
    assert_eq!(anonymous.json["code"], "unauthenticated");
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn unsafe_requests_need_csrf_token_and_origin(pool: PgPool) {
    let app = TestApp::new(pool);
    let auth = app.professor("prof_1@example.com").await;
    let room = json!({"title": "Lab 1", "mode": "assignment"});

    let no_csrf = req("POST", "/api/v1/p/rooms")
        .header("cookie", &auth.cookie)
        .header("origin", ORIGIN);
    let wrong_csrf = req("POST", "/api/v1/p/rooms")
        .header("cookie", &auth.cookie)
        .header("x-csrf-token", "nope")
        .header("origin", ORIGIN);
    let wrong_origin = req("POST", "/api/v1/p/rooms")
        .header("cookie", &auth.cookie)
        .header("x-csrf-token", &auth.csrf)
        .header("origin", "https://evil.example.net");
    let no_origin = req("POST", "/api/v1/p/rooms")
        .header("cookie", &auth.cookie)
        .header("x-csrf-token", &auth.csrf);

    for builder in [no_csrf, wrong_csrf, wrong_origin, no_origin] {
        let r = app.send(builder, Some(room.clone())).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN);
        assert_eq!(r.json["code"], "forbidden");
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM rooms")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn logout_ends_the_session(pool: PgPool) {
    let app = TestApp::new(pool);
    let auth = app.professor("prof_1@example.com").await;
    let r = app
        .send(authed("POST", "/api/v1/p/logout", &auth), None)
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let cleared = r.headers["set-cookie"].to_str().unwrap();
    assert!(cleared.contains("Max-Age=0"));

    let after = app
        .send(authed("GET", "/api/v1/p/session", &auth), None)
        .await;
    assert_eq!(after.status, StatusCode::UNAUTHORIZED);
}

async fn status(app: &TestApp, auth: &Auth) -> StatusCode {
    app.send(authed("GET", "/api/v1/p/session", auth), None)
        .await
        .status
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn password_change_revokes_other_sessions(pool: PgPool) {
    let app = TestApp::new(pool);
    let a = app.professor("prof_1@example.com").await;
    let b = app.login_ok("prof_1@example.com").await;

    let wrong = app
        .send(
            authed("POST", "/api/v1/p/password", &a),
            Some(json!({"current_password": "not my password", "new_password": NEW_PASSWORD})),
        )
        .await;
    assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(wrong.json["errors"][0]["field"], "current_password");

    let r = app
        .send(
            authed("POST", "/api/v1/p/password", &a),
            Some(json!({"current_password": PASSWORD, "new_password": NEW_PASSWORD})),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(status(&app, &a).await, StatusCode::OK);
    assert_eq!(status(&app, &b).await, StatusCode::UNAUTHORIZED);

    let old = app.login("prof_1@example.com", PASSWORD).await;
    assert_eq!(old.status, StatusCode::UNAUTHORIZED);
    let new = app.login("prof_1@example.com", NEW_PASSWORD).await;
    assert_eq!(new.status, StatusCode::OK);
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn sessions_can_be_listed_and_revoked(pool: PgPool) {
    let app = TestApp::new(pool);
    let a = app.professor("prof_1@example.com").await;
    let b = app.login_ok("prof_1@example.com").await;
    let other = app.professor("prof_2@example.com").await;

    let list = app
        .send(authed("GET", "/api/v1/p/sessions", &a), None)
        .await;
    assert_eq!(list.status, StatusCode::OK);
    let items = list.json["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items.iter().filter(|s| s["current"] == true).count(), 1);
    let b_id = items.iter().find(|s| s["current"] == false).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let foreign = app
        .send(
            authed("DELETE", &format!("/api/v1/p/sessions/{b_id}"), &other),
            None,
        )
        .await;
    assert_eq!(foreign.status, StatusCode::NOT_FOUND);
    assert_eq!(status(&app, &b).await, StatusCode::OK);

    let r = app
        .send(
            authed("DELETE", &format!("/api/v1/p/sessions/{b_id}"), &a),
            None,
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(status(&app, &b).await, StatusCode::UNAUTHORIZED);
    assert_eq!(status(&app, &a).await, StatusCode::OK);
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn idle_and_absolute_limits_end_sessions(pool: PgPool) {
    let app = TestApp::new(pool);
    let a = app.professor("prof_1@example.com").await;
    let b = app.login_ok("prof_1@example.com").await;
    let hash = |auth: &Auth| api::token::sha256(auth.cookie.strip_prefix("__Host-ps=").unwrap());

    sqlx::query("UPDATE professor_sessions SET last_seen_at = now() - interval '13 hours' WHERE id_hash = $1")
        .bind(hash(&a))
        .execute(&app.pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE professor_sessions SET expires_at = now() - interval '1 second' WHERE id_hash = $1",
    )
    .bind(hash(&b))
    .execute(&app.pool)
    .await
    .unwrap();
    assert_eq!(status(&app, &a).await, StatusCode::UNAUTHORIZED);
    assert_eq!(status(&app, &b).await, StatusCode::UNAUTHORIZED);
}
