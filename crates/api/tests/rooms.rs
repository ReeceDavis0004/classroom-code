mod common;

use axum::http::StatusCode;
use common::{Auth, ORIGIN, TestApp, authed};
use serde_json::{Value, json};
use sqlx::PgPool;

async fn create(app: &TestApp, auth: &Auth, title: &str, mode: &str) -> common::Resp {
    app.send(
        authed("POST", "/api/v1/p/rooms", auth),
        Some(json!({"title": title, "mode": mode})),
    )
    .await
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn create_and_read_rooms(pool: PgPool) {
    let app = TestApp::new(pool);
    let auth = app.professor("prof_1@example.com").await;

    let r = create(&app, &auth, "Lab 1", "assignment").await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert_eq!(r.json["mode"], "assignment");
    assert_eq!(r.json["status"], "open");
    assert_eq!(r.json["game_state"], Value::Null);
    let share_url = r.json["share_url"].as_str().unwrap();
    let token = share_url.strip_prefix(&format!("{ORIGIN}/join/")).unwrap();
    assert_eq!(token.len(), 24);
    assert!(
        token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    );
    let created_at = r.json["created_at"].as_str().unwrap();
    assert_eq!(created_at.len(), 24, "{created_at}");
    assert!(created_at.ends_with('Z'));

    let id = r.json["id"].as_str().unwrap();
    assert_eq!(id.as_bytes()[14], b'7');
    let got = app
        .send(authed("GET", &format!("/api/v1/p/rooms/{id}"), &auth), None)
        .await;
    assert_eq!(got.status, StatusCode::OK);
    assert_eq!(got.json, r.json);

    let game = create(&app, &auth, "Quiz", "game").await;
    assert_eq!(game.json["game_state"], "lobby");

    let bad = create(&app, &auth, "Quiz", "party").await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(bad.json["errors"][0]["field"], "mode");
    let empty = create(&app, &auth, "  ", "game").await;
    assert_eq!(empty.json["errors"][0]["field"], "title");
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn list_paginates_newest_first(pool: PgPool) {
    let app = TestApp::new(pool);
    let auth = app.professor("prof_1@example.com").await;
    for i in 1..=5 {
        create(&app, &auth, &format!("Lab {i}"), "assignment").await;
    }

    let mut titles = Vec::new();
    let mut cursor = String::new();
    let mut pages = 0;
    loop {
        let r = app
            .send(
                authed(
                    "GET",
                    &format!("/api/v1/p/rooms?limit=2&cursor={cursor}"),
                    &auth,
                ),
                None,
            )
            .await;
        assert_eq!(r.status, StatusCode::OK);
        pages += 1;
        for room in r.json["items"].as_array().unwrap() {
            titles.push(room["title"].as_str().unwrap().to_string());
        }
        match r.json["next_cursor"].as_str() {
            Some(next) => cursor = next.to_string(),
            None => break,
        }
    }
    assert_eq!(pages, 3);
    assert_eq!(titles, ["Lab 5", "Lab 4", "Lab 3", "Lab 2", "Lab 1"]);

    let all = app
        .send(authed("GET", "/api/v1/p/rooms", &auth), None)
        .await;
    assert_eq!(all.json["items"].as_array().unwrap().len(), 5);
    assert_eq!(all.json["next_cursor"], Value::Null);

    let bad = app
        .send(authed("GET", "/api/v1/p/rooms?limit=0", &auth), None)
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrator = "classroom_core::MIGRATOR")]
async fn other_professors_room_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool);
    let owner = app.professor("prof_1@example.com").await;
    let other = app.professor("prof_2@example.com").await;
    let room = create(&app, &owner, "Lab 1", "assignment").await;
    let id = room.json["id"].as_str().unwrap();

    let r = app
        .send(
            authed("GET", &format!("/api/v1/p/rooms/{id}"), &other),
            None,
        )
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.json["code"], "not_found");

    let list = app
        .send(authed("GET", "/api/v1/p/rooms", &other), None)
        .await;
    assert!(list.json["items"].as_array().unwrap().is_empty());

    let garbage = app
        .send(authed("GET", "/api/v1/p/rooms/not-a-uuid", &other), None)
        .await;
    assert_eq!(garbage.status, StatusCode::NOT_FOUND);
}
