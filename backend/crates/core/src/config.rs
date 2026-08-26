use figment::{
    Figment,
    providers::{Format, Toml, Env},
};
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub elasticsearch: ElasticsearchConfig,
    pub oidc: OidcConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub addr: SocketAddr,
    pub worker_threads: usize,
}

impl std::fmt::Display for ServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.addr)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub enable_wal: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ElasticsearchConfig {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub index_prefix: String,
    pub request_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OidcConfig {
    pub issuer_url: String,
    pub realm: Option<String>,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    pub jwt_secret: String,
}

impl AppConfig {
    pub fn load() -> Result<Self, figment::Error> {
        Figment::from(
            Figment::new()
                .merge(Toml::file("config.toml"))
                .merge(Env::prefixed("AADS_").split("__"))
        ).extract()
    }
}
