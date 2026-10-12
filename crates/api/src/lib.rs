use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
};
use sqlx::PgPool;

pub mod account;
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

    let professor = Router::new()
        .route("/session", get(account::session))
        .route("/logout", post(account::logout))
        .route("/password", post(account::change_password))
        .route("/sessions", get(account::list_sessions))
        .route(
            "/sessions/{id}",
            axum::routing::delete(account::revoke_session),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            session::require,
        ));

    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .nest("/api/v1/pub", public)
        .nest("/api/v1/p", professor)
        .fallback(|| async { error::ApiError::NotFound })
        .layer(DefaultBodyLimit::max(256 * 1024))
        .with_state(state)
}
