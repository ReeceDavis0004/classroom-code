use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use sqlx::PgPool;

pub mod config;
pub mod error;
pub mod invite;
pub mod login;
pub mod mail;
pub mod password;
pub mod register;
pub mod reset;
pub mod session;
pub mod time;
pub mod token;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub site_origin: Arc<str>,
    pub mailer: mail::Mailer,
}

pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/auth/register", post(register::register))
        .route("/auth/verify", post(register::verify))
        .route("/auth/login", post(login::login))
        .route("/auth/reset-request", post(reset::request))
        .route("/auth/reset", post(reset::reset));

    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .nest("/api/v1/pub", public)
        .fallback(|| async { error::ApiError::NotFound })
        .layer(DefaultBodyLimit::max(256 * 1024))
        .with_state(state)
}
