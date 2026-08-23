use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Run migrations manually
    sqlx::query(include_str!("../../../migrations/001_initial_schema.sql"))
        .execute(&pool)
        .await?;

    sqlx::query(include_str!("../../../migrations/002_seed_rules.sql"))
        .execute(&pool)
        .await?;

    Ok(pool)
}
