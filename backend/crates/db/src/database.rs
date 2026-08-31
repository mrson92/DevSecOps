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

    sqlx::query(include_str!("../../../migrations/003_system_settings.sql"))
        .execute(&pool)
        .await?;

    sqlx::query(include_str!("../../../migrations/004_fix_seed_rules.sql"))
        .execute(&pool)
        .await?;

    sqlx::query(include_str!("../../../migrations/005_ai_agents.sql"))
        .execute(&pool)
        .await?;

    sqlx::query(include_str!("../../../migrations/006_unique_report_period.sql"))
        .execute(&pool)
        .await?;

    // Idempotent column add: SQLite has no "ADD COLUMN IF NOT EXISTS", and
    // migrations run on every startup, so only add `tags` when missing.
    let has_tags: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM pragma_table_info('rules') WHERE name = 'tags'"
    )
    .fetch_one(&pool)
    .await?;
    if has_tags.0 == 0 {
        sqlx::query(include_str!("../../../migrations/007_rule_tags.sql"))
            .execute(&pool)
            .await?;
    }

    // Batch-2 MITRE-mapped rules (INSERT OR IGNORE -> idempotent on every startup).
    sqlx::query(include_str!("../../../migrations/008_add_mitre_rules.sql"))
        .execute(&pool)
        .await?;

    // 2차 LLM 위협 분석 전용 페르소나 (INSERT OR IGNORE -> idempotent).
    sqlx::query(include_str!("../../../migrations/009_threat_analyst_persona.sql"))
        .execute(&pool)
        .await?;

    Ok(pool)
}
