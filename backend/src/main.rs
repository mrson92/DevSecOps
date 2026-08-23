use anyhow::Result;
use axum::{routing::get, Router};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use aads_core::config::AppConfig;
use aads_core::state::{AppState, ElasticSearchClientTrait};
use aads_es::client::ElasticSearchClient;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "aads_backend=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting AADS Backend...");

    let config = AppConfig::load().map_err(|e| anyhow::anyhow!("Config error: {}", e))?;
    tracing::info!("Config loaded: {}", config.server.addr);

    let db_pool = aads_db::database::create_pool(&config.database.url).await?;
    tracing::info!("Database connected");

    let es_client = ElasticSearchClient::new(&config.elasticsearch)?;
    tracing::info!("ElasticSearch client created");

    let app_state = AppState {
        db: db_pool,
        es: Arc::new(es_client) as Arc<dyn ElasticSearchClientTrait>,
        config: Arc::new(config.clone()),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/rules", get(aads_api::handlers::rules::list_rules))
        .route("/api/v1/rules/{id}", get(aads_api::handlers::rules::get_rule))
        .route("/api/v1/detections", get(aads_api::handlers::detections::list_detections))
        .route("/api/v1/dashboard/stats", get(aads_api::handlers::dashboard::get_stats))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(&config.server.addr).await?;
    tracing::info!("Listening on {}", config.server.addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
