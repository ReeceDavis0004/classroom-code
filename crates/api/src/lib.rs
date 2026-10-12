use std::sync::Arc;

use axum::{Router, extract::DefaultBodyLimit, routing::get};
use sqlx::PgPool;

pub mod config;
pub mod error;
pub mod invite;
pub mod mail;
pub mod password;
pub mod time;
pub mod token;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub site_origin: Arc<str>,
    pub mailer: mail::Mailer,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .fallback(|| async { error::ApiError::NotFound })
        .layer(DefaultBodyLimit::max(256 * 1024))
        .with_state(state)
}
