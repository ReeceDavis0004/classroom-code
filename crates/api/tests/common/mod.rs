#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use api::{AppState, mail::Sent};
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, request::Builder},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

pub const ORIGIN: &str = "https://class.example.com";
pub const PASSWORD: &str = "correct horse battery";

pub struct TestApp {
    pub pool: PgPool,
    router: Router,
    mail: Arc<Mutex<Vec<Sent>>>,
}

pub struct Auth {
    pub cookie: String,
    pub csrf: String,
}

pub struct Resp {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub json: Value,
}

pub fn req(method: &str, path: &str) -> Builder {
    Request::builder().method(method).uri(path)
}

pub fn authed(method: &str, path: &str, auth: &Auth) -> Builder {
    req(method, path)
        .header("cookie", &auth.cookie)
        .header("x-csrf-token", &auth.csrf)
        .header("origin", ORIGIN)
}

impl TestApp {
    pub fn new(pool: PgPool) -> Self {
        let (mailer, mail) = api::mail::Mailer::memory();
        let router = api::router(AppState {
            pool: pool.clone(),
            site_origin: ORIGIN.into(),
            mailer,
        });
        Self { pool, router, mail }
    }

    pub async fn send(&self, builder: Builder, body: Option<Value>) -> Resp {
        let request = match body {
            Some(body) => builder
                .header("content-type", "application/json")
                .body(Body::from(body.to_string())),
            None => builder.body(Body::empty()),
        }
        .unwrap();
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Resp {
            status,
            headers,
            json,
        }
    }

    pub async fn post(&self, path: &str, body: Value) -> Resp {
        self.send(req("POST", path), Some(body)).await
    }

    pub fn mails(&self) -> Vec<Sent> {
        self.mail.lock().unwrap().clone()
    }

    pub fn last_token(&self) -> String {
        let mails = self.mails();
        let body = &mails.last().expect("no email sent").body;
        let start = body.find("token=").expect("no token in email") + "token=".len();
        body[start..].split_whitespace().next().unwrap().to_string()
    }

    pub async fn invite(&self) -> String {
        api::invite::create(&self.pool).await.unwrap()
    }

    pub async fn register(&self, email: &str) -> Resp {
        let code = self.invite().await;
        self.post(
            "/api/v1/pub/auth/register",
            json!({"invite_code": code, "email": email, "display_name": "prof_1", "password": PASSWORD}),
        )
        .await
    }

    pub async fn register_verified(&self, email: &str) {
        assert_eq!(self.register(email).await.status, StatusCode::CREATED);
        let token = self.last_token();
        let r = self
            .post("/api/v1/pub/auth/verify", json!({"token": token}))
            .await;
        assert_eq!(r.status, StatusCode::NO_CONTENT);
    }

    pub async fn login(&self, email: &str, password: &str) -> Resp {
        self.post(
            "/api/v1/pub/auth/login",
            json!({"email": email, "password": password}),
        )
        .await
    }

    pub async fn login_ok(&self, email: &str) -> Auth {
        let r = self.login(email, PASSWORD).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json);
        let set_cookie = r.headers["set-cookie"].to_str().unwrap();
        Auth {
            cookie: set_cookie.split(';').next().unwrap().to_string(),
            csrf: r.json["csrf_token"].as_str().unwrap().to_string(),
        }
    }

    pub async fn professor(&self, email: &str) -> Auth {
        self.register_verified(email).await;
        self.login_ok(email).await
    }
}
