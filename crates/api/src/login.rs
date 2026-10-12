use axum::{
    extract::State,
    http::header,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    error::{ApiError, Json},
    password, session,
};

#[derive(Deserialize)]
pub struct Login {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct Professor {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct SessionInfo {
    pub professor: Professor,
    pub csrf_token: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<Login>,
) -> Result<Response, ApiError> {
    let row = sqlx::query!(
        r#"SELECT id, email::text AS "email!", display_name, status, password_hash
             FROM professors WHERE email = $1::text::citext"#,
        body.email.trim(),
    )
    .fetch_optional(&state.pool)
    .await?;

    let stored = row.as_ref().map(|r| r.password_hash.clone());
    let password_ok = password::verify(body.password, stored).await;
    let row = match row {
        Some(row) if password_ok && row.status == "active" => row,
        _ => return Err(ApiError::Unauthenticated),
    };

    let new = session::create(&state.pool, row.id).await?;
    let info = SessionInfo {
        professor: Professor {
            id: row.id,
            email: row.email,
            display_name: row.display_name,
            status: row.status,
        },
        csrf_token: new.csrf_token,
    };
    Ok(([(header::SET_COOKIE, new.cookie)], axum::Json(info)).into_response())
}
