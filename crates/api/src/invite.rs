use sqlx::PgPool;

use crate::token;

pub async fn create(pool: &PgPool) -> sqlx::Result<String> {
    let code = token::random(18);
    sqlx::query!(
        "INSERT INTO professor_invites (code_hash, expires_at) VALUES ($1, now() + interval '7 days')",
        token::sha256(&code),
    )
    .execute(pool)
    .await?;
    Ok(code)
}
