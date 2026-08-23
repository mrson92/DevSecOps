use crate::config::AppConfig;
use crate::error::AppError;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::SqlitePool;
use std::sync::Arc;

#[async_trait]
pub trait ElasticSearchClientTrait: Send + Sync {
    async fn search(&self, index: &str, query: Value) -> Result<Value, AppError>;
    async fn health_check(&self) -> Result<bool, AppError>;
}

pub struct AppState {
    pub db: SqlitePool,
    pub es: Arc<dyn ElasticSearchClientTrait>,
    pub config: Arc<AppConfig>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            es: self.es.clone(),
            config: self.config.clone(),
        }
    }
}
