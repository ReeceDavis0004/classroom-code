use axum::{
    Extension,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    error::{ApiError, Json},
    login::{Professor, SessionInfo},
    password,
    session::{self, Session},
};

pub async fn session(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<axum::Json<SessionInfo>, ApiError> {
    let professor = sqlx::query_as!(
        Professor,
        r#"SELECT id, email::text AS "email!", display_name, status FROM professors WHERE id = $1"#,
        session.professor_id,
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(axum::Json(SessionInfo {
        professor,
        csrf_token: session.csrf_token,
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Response, ApiError> {
    sqlx::query!(
        "UPDATE professor_sessions SET revoked_at = now() WHERE public_id = $1",
        session.public_id,
    )
    .execute(&state.pool)
    .await?;
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, session::clear_cookie())],
    )
        .into_response())
}

#[derive(Deserialize)]
pub struct ChangePassword {
    current_password: String,
    new_password: String,
}

pub async fn change_password(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
    Json(body): Json<ChangePassword>,
) -> Result<StatusCode, ApiError> {
    password::check("new_password", &body.new_password)?;
    let stored = sqlx::query_scalar!(
        "SELECT password_hash FROM professors WHERE id = $1",
        session.professor_id,
    )
    .fetch_one(&state.pool)
    .await?;
    if !password::verify(body.current_password, Some(stored)).await {
        return Err(ApiError::field(
            "current_password",
            "incorrect",
            "The current password is incorrect",
        ));
    }
    let password_hash = password::hash(body.new_password).await?;

    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "UPDATE professors SET password_hash = $1 WHERE id = $2",
        password_hash,
        session.professor_id,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE professor_sessions SET revoked_at = now()
          WHERE professor_id = $1 AND public_id <> $2 AND revoked_at IS NULL",
        session.professor_id,
        session.public_id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct SessionItem {
    id: Uuid,
    #[serde(serialize_with = "crate::time::ms")]
    created_at: DateTime<Utc>,
    #[serde(serialize_with = "crate::time::ms")]
    last_seen_at: DateTime<Utc>,
    #[serde(serialize_with = "crate::time::ms")]
    expires_at: DateTime<Utc>,
    current: bool,
}

#[derive(Serialize)]
pub struct SessionList {
    items: Vec<SessionItem>,
    next_cursor: Option<String>,
}

pub async fn list_sessions(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<axum::Json<SessionList>, ApiError> {
    let items = sqlx::query_as!(
        SessionItem,
        r#"SELECT public_id AS id, created_at, last_seen_at, expires_at,
                  public_id = $2 AS "current!"
             FROM professor_sessions
            WHERE professor_id = $1 AND revoked_at IS NULL AND expires_at > now()
              AND last_seen_at > now() - interval '12 hours'
            ORDER BY created_at DESC"#,
        session.professor_id,
        session.public_id,
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(axum::Json(SessionList {
        items,
        next_cursor: None,
    }))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id: Uuid = id.parse().map_err(|_| ApiError::NotFound)?;
    let revoked = sqlx::query!(
        "UPDATE professor_sessions SET revoked_at = now()
          WHERE public_id = $1 AND professor_id = $2 AND revoked_at IS NULL",
        id,
        session.professor_id,
    )
    .execute(&state.pool)
    .await?;
    if revoked.rows_affected() != 1 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
