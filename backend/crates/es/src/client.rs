use aads_core::error::AppError;
use aads_core::state::ElasticSearchClientTrait;
use async_trait::async_trait;
use elasticsearch::{
    http::transport::Transport,
    cluster::ClusterHealthParts,
    Elasticsearch,
    SearchParts,
};
use serde_json::Value;

use aads_core::config::ElasticsearchConfig;

pub struct ElasticSearchClient {
    client: Elasticsearch,
    index_prefix: String,
}

impl ElasticSearchClient {
    pub fn new(config: &ElasticsearchConfig) -> Result<Self, AppError> {
        let transport = Transport::single_node(&config.url)
            .map_err(|e| AppError::Config(format!("ES transport error: {}", e)))?;

        let client = Elasticsearch::new(transport);

        Ok(Self {
            client,
            index_prefix: config.index_prefix.clone(),
        })
    }

    fn full_index_name(&self, suffix: &str) -> String {
        format!("{}-{}", self.index_prefix, suffix)
    }
}

#[async_trait]
impl ElasticSearchClientTrait for ElasticSearchClient {
    async fn search(&self, index: &str, query: Value) -> Result<Value, AppError> {
        let full_index = self.full_index_name(index);

        let response = self
            .client
            .search(SearchParts::Index(&[&full_index]))
            .body(query)
            .send()
            .await
            .map_err(|e| AppError::ElasticSearch(format!("Search error: {}", e)))?;

        let response_body = response
            .json::<Value>()
            .await
            .map_err(|e| AppError::ElasticSearch(format!("Response parse error: {}", e)))?;

        Ok(response_body)
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        let response = self
            .client
            .cluster()
            .health(ClusterHealthParts::None)
            .send()
            .await
            .map_err(|e| AppError::ElasticSearch(format!("Health check error: {}", e)))?;

        Ok(response.status_code().is_success())
    }
}
