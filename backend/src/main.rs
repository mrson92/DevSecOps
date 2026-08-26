use anyhow::Result;
use axum::{routing::get, Router};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use aads_core::config::AppConfig;
use aads_core::state::{AppState, ElasticSearchClientTrait};
use aads_es::client::ElasticSearchClient;
use aads_engine::Scheduler;

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

    let scheduler_state = app_state.clone();
    let scheduler = Scheduler::new(scheduler_state, 60, 3);
    tokio::spawn(async move {
        scheduler.start().await;
    });
    tracing::info!("Scheduler started (60s interval, max 3 concurrent)");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/rules", get(aads_api::handlers::rules::list_rules).post(aads_api::handlers::rules::create_rule))
        .route("/api/v1/rules/{id}", get(aads_api::handlers::rules::get_rule).put(aads_api::handlers::rules::update_rule).delete(aads_api::handlers::rules::delete_rule))
        .route("/api/v1/rules/{id}/test", axum::routing::post(aads_api::handlers::rules::test_rule))
        .route("/api/v1/detections", get(aads_api::handlers::detections::list_detections))
        .route("/api/v1/detections/{id}", get(aads_api::handlers::detections::get_detection).patch(aads_api::handlers::detections::update_detection))
        .route("/api/v1/dashboard/stats", get(aads_api::handlers::dashboard::get_stats))
        .route("/api/v1/dashboard/timeline", get(aads_api::handlers::dashboard::get_timeline))
        .route("/api/v1/dashboard/top-rules", get(aads_api::handlers::dashboard::get_top_rules))
        .route("/api/v1/dashboard/top-ips", get(aads_api::handlers::dashboard::get_top_ips))
        .route("/api/v1/engine/run", axum::routing::post(aads_api::handlers::engine::run_rules))
        .route("/api/v1/engine/run/{rule_id}", axum::routing::post(aads_api::handlers::engine::run_single_rule))
        .route("/api/v1/reports", get(aads_api::handlers::reports::list_reports).post(aads_api::handlers::reports::generate_report))
        .route("/api/v1/reports/{id}", get(aads_api::handlers::reports::get_report))
        .route("/api/v1/data-sources", get(aads_api::handlers::settings::list_data_sources).post(aads_api::handlers::settings::create_data_source))
        .route("/api/v1/data-sources/{id}", axum::routing::delete(aads_api::handlers::settings::delete_data_source).put(aads_api::handlers::settings::update_data_source))
        .route("/api/v1/data-sources/{id}/test", axum::routing::post(aads_api::handlers::settings::test_data_source))
        .route("/api/v1/notifications/channels", get(aads_api::handlers::settings::list_notification_channels).post(aads_api::handlers::settings::create_notification_channel))
        .route("/api/v1/notifications/channels/{id}", axum::routing::delete(aads_api::handlers::settings::delete_notification_channel))
        .route("/api/v1/notifications/channels/{id}/test", axum::routing::post(aads_api::handlers::settings::test_notification_channel))
        .route("/api/v1/settings/oidc", get(aads_api::handlers::settings::get_oidc_settings).put(aads_api::handlers::settings::update_oidc_settings))
        .route("/api/v1/settings/oidc/test", axum::routing::post(aads_api::handlers::settings::test_oidc_connection))
        .route("/api/v1/agents", get(aads_api::handlers::agents::list_agents).post(aads_api::handlers::agents::create_agent))
        .route("/api/v1/agents/{id}", get(aads_api::handlers::agents::get_agent).put(aads_api::handlers::agents::update_agent).delete(aads_api::handlers::agents::delete_agent))
        .route("/api/v1/personas", get(aads_api::handlers::personas::list_personas).post(aads_api::handlers::personas::create_persona))
        .route("/api/v1/personas/{id}", get(aads_api::handlers::personas::get_persona).put(aads_api::handlers::personas::update_persona).delete(aads_api::handlers::personas::delete_persona))
        .route("/api/v1/auth/me", get(aads_api::handlers::auth::get_current_user))
        .route("/api/v1/auth/oidc/login", get(aads_api::handlers::auth::oidc_login))
        .route("/api/v1/auth/oidc/callback", axum::routing::post(aads_api::handlers::auth::oidc_callback))
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
