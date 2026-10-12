use axum::{extract::State, http::StatusCode};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    error::{ApiError, FieldError, Json},
    login::Professor,
    password, token,
};

#[derive(Deserialize)]
pub struct Register {
    invite_code: String,
    email: String,
    display_name: String,
    password: String,
}

fn field(field: &'static str, code: &'static str, message: &str) -> FieldError {
    FieldError {
        field,
        code,
        message: message.into(),
    }
}

fn validate(email: &str, display_name: &str, pw: &str) -> Result<(), ApiError> {
    let mut errors = Vec::new();
    let valid_email = email.len() <= 254
        && !email.contains(char::is_whitespace)
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    if !valid_email {
        errors.push(field("email", "invalid", "Enter a valid email address"));
    }
    if display_name.is_empty() || display_name.chars().count() > 100 {
        errors.push(field("display_name", "invalid", "Use 1 to 100 characters"));
    }
    if let Err(ApiError::Validation(mut e)) = password::check("password", pw) {
        errors.append(&mut e);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ApiError::Validation(errors))
    }
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<Register>,
) -> Result<(StatusCode, axum::Json<Professor>), ApiError> {
    let email = body.email.trim();
    let display_name = body.display_name.trim();
    validate(email, display_name, &body.password)?;
    let password_hash = password::hash(body.password).await?;

    let mut tx = state.pool.begin().await?;
    let invite = sqlx::query!(
        "UPDATE professor_invites SET used_at = now()
          WHERE code_hash = $1 AND used_at IS NULL AND expires_at > now()",
        token::sha256(body.invite_code.trim()),
    )
    .execute(&mut *tx)
    .await?;
    if invite.rows_affected() != 1 {
        return Err(ApiError::field(
            "invite_code",
            "invalid",
            "This invite code is invalid, expired or already used",
        ));
    }

    let id = Uuid::now_v7();
    let inserted = sqlx::query!(
        "INSERT INTO professors (id, email, password_hash, display_name)
         VALUES ($1, $2::text::citext, $3, $4)
         ON CONFLICT (email) DO NOTHING",
        id,
        email,
        password_hash,
        display_name,
    )
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() != 1 {
        return Err(ApiError::field(
            "email",
            "taken",
            "An account with this email already exists",
        ));
    }

    let verify_token = token::random(32);
    sqlx::query!(
        "INSERT INTO email_tokens (token_hash, professor_id, purpose, expires_at)
         VALUES ($1, $2, 'verify', now() + interval '24 hours')",
        token::sha256(&verify_token),
        id,
    )
    .execute(&mut *tx)
    .await?;

    let text = format!(
        "Confirm your email address to finish creating your account:\n\n{}/verify?token={verify_token}\n\nThe link expires in 24 hours.\n",
        state.site_origin
    );
    state
        .mailer
        .send(email, "Confirm your email address", text)
        .await
        .map_err(ApiError::Internal)?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        axum::Json(Professor {
            id,
            email: email.into(),
            display_name: display_name.into(),
            status: "unverified".into(),
        }),
    ))
}

#[derive(Deserialize)]
pub struct Verify {
    token: String,
}

pub async fn verify(
    State(state): State<AppState>,
    Json(body): Json<Verify>,
) -> Result<StatusCode, ApiError> {
    let verified = sqlx::query!(
        "WITH t AS (
           UPDATE email_tokens SET used_at = now()
            WHERE token_hash = $1 AND purpose = 'verify' AND used_at IS NULL AND expires_at > now()
           RETURNING professor_id)
         UPDATE professors p SET status = 'active'
           FROM t WHERE p.id = t.professor_id AND p.status = 'unverified'",
        token::sha256(&body.token),
    )
    .execute(&state.pool)
    .await?;
    if verified.rows_affected() != 1 {
        return Err(ApiError::field(
            "token",
            "invalid",
            "This link is invalid, expired or already used",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}
