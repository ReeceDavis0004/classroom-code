use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is not set")?;
    let pool = PgPool::connect(&url).await?;
    classroom_core::MIGRATOR.run(&pool).await?;
    println!("migrations applied");
    Ok(())
}
