use axum::{extract::State, http::StatusCode};
use serde::Deserialize;

use crate::{
    AppState,
    error::{ApiError, Json},
    password, token,
};

#[derive(Deserialize)]
pub struct ResetRequest {
    email: String,
}

pub async fn request(
    State(state): State<AppState>,
    Json(body): Json<ResetRequest>,
) -> Result<StatusCode, ApiError> {
    let reset_token = token::random(32);
    let email = sqlx::query_scalar!(
        r#"WITH p AS (
             SELECT id, email::text AS email FROM professors
              WHERE email = $2::text::citext AND status = 'active'),
           t AS (
             INSERT INTO email_tokens (token_hash, professor_id, purpose, expires_at)
             SELECT $1, id, 'reset', now() + interval '30 minutes' FROM p)
         SELECT email AS "email!" FROM p"#,
        token::sha256(&reset_token),
        body.email.trim(),
    )
    .fetch_optional(&state.pool)
    .await?;

    if let Some(email) = email {
        let text = format!(
            "Someone asked to reset the password for this account. If it was you, open:\n\n{}/reset?token={reset_token}\n\nThe link expires in 30 minutes and works once. If it was not you, ignore this email.\n",
            state.site_origin
        );
        if let Err(e) = state.mailer.send(&email, "Reset your password", text).await {
            eprintln!("reset email failed: {e}");
        }
    }
    Ok(StatusCode::ACCEPTED)
}

#[derive(Deserialize)]
pub struct Reset {
    token: String,
    password: String,
}

pub async fn reset(
    State(state): State<AppState>,
    Json(body): Json<Reset>,
) -> Result<StatusCode, ApiError> {
    password::check("password", &body.password)?;
    let password_hash = password::hash(body.password).await?;

    let mut tx = state.pool.begin().await?;
    let professor_id = sqlx::query_scalar!(
        "UPDATE email_tokens SET used_at = now()
          WHERE token_hash = $1 AND purpose = 'reset' AND used_at IS NULL AND expires_at > now()
         RETURNING professor_id",
        token::sha256(&body.token),
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::field(
            "token",
            "invalid",
            "This link is invalid, expired or already used",
        )
    })?;

    sqlx::query!(
        "UPDATE professors SET password_hash = $1 WHERE id = $2",
        password_hash,
        professor_id,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE professor_sessions SET revoked_at = now()
          WHERE professor_id = $1 AND revoked_at IS NULL",
        professor_id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
