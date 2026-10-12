use std::error::Error;

use api::{AppState, config::Config, mail::Mailer};
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let command = std::env::args().nth(1);
    let database_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is not set")?;
    let pool = PgPool::connect(&database_url).await?;
    if !classroom_core::schema_is_current(&pool).await? {
        return Err("database schema is older than this build; run migrate first".into());
    }

    match command.as_deref() {
        None | Some("serve") => serve(pool).await,
        Some(other) => Err(format!("unknown command: {other}").into()),
    }
}

async fn serve(pool: PgPool) -> Result<(), Box<dyn Error>> {
    let config = Config::from_env()?;
    let state = AppState {
        pool,
        site_origin: config.site_origin.into(),
        mailer: Mailer::smtp(&config.smtp_url, &config.mail_from)?,
    };
    let listener = tokio::net::TcpListener::bind(&config.api_addr).await?;
    axum::serve(listener, api::router(state)).await?;
    Ok(())
}
