use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, header},
    middleware::Next,
    response::Response,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{AppState, error::ApiError, token};

pub const COOKIE: &str = "__Host-ps";
const MAX_AGE_S: u32 = 7 * 24 * 3600;

#[derive(Clone)]
pub struct Session {
    pub public_id: Uuid,
    pub professor_id: Uuid,
    pub csrf_token: String,
}

pub struct NewSession {
    pub cookie: HeaderValue,
    pub csrf_token: String,
}

pub async fn create(pool: &PgPool, professor_id: Uuid) -> Result<NewSession, ApiError> {
    let secret = token::random(32);
    let csrf_token = token::random(32);
    sqlx::query!(
        "INSERT INTO professor_sessions (id_hash, public_id, professor_id, csrf_token, expires_at)
         VALUES ($1, $2, $3, $4, now() + interval '7 days')",
        token::sha256(&secret),
        Uuid::now_v7(),
        professor_id,
        csrf_token,
    )
    .execute(pool)
    .await?;
    let cookie =
        format!("{COOKIE}={secret}; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age={MAX_AGE_S}");
    Ok(NewSession {
        cookie: HeaderValue::from_str(&cookie).expect("cookie is ASCII"),
        csrf_token,
    })
}

pub fn clear_cookie() -> HeaderValue {
    HeaderValue::from_static("__Host-ps=; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=0")
}

fn cookie_secret(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE)
        .map(|(_, value)| value)
}

pub async fn require(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let secret = cookie_secret(req.headers()).ok_or(ApiError::Unauthenticated)?;
    let session = sqlx::query_as!(
        Session,
        "UPDATE professor_sessions s SET last_seen_at = now()
           FROM professors p
          WHERE s.id_hash = $1 AND p.id = s.professor_id AND p.status = 'active'
            AND s.revoked_at IS NULL AND s.expires_at > now()
            AND s.last_seen_at > now() - interval '12 hours'
         RETURNING s.public_id, s.professor_id, s.csrf_token",
        token::sha256(secret),
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::Unauthenticated)?;

    if !req.method().is_safe() {
        let header = |name| req.headers().get(name).and_then(|v| v.to_str().ok());
        let origin_ok = header("origin") == Some(&*state.site_origin);
        let csrf_ok = header("x-csrf-token").is_some_and(|t| token::eq(t, &session.csrf_token));
        if !origin_ok || !csrf_ok {
            return Err(ApiError::Forbidden);
        }
    }

    req.extensions_mut().insert(session);
    Ok(next.run(req).await)
}
