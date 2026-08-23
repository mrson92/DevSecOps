use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("ElasticSearch error: {0}")]
    ElasticSearch(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Rule engine error: {0}")]
    RuleEngine(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::ElasticSearch(e) => (StatusCode::BAD_GATEWAY, e.clone()),
            AppError::Config(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
            AppError::Validation(e) => (StatusCode::BAD_REQUEST, e.clone()),
            AppError::NotFound(e) => (StatusCode::NOT_FOUND, e.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string()),
            AppError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
            AppError::RuleEngine(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
        };

        let body = json!({
            "success": false,
            "error": {
                "code": "APP_ERROR",
                "message": message
            }
        });

        (status, axum::Json(body)).into_response()
    }
}
