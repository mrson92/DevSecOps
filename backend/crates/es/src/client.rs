use aads_core::error::AppError;
use aads_core::state::ElasticSearchClientTrait;
use async_trait::async_trait;
use elasticsearch::{
    http::transport::Transport,
    cluster::ClusterHealthParts,
    indices::{IndicesCreateParts, IndicesExistsParts},
    BulkParts,
    Elasticsearch,
    IndexParts,
    SearchParts,
};
use serde_json::{json, Value};

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

    async fn index_document(&self, index: &str, id: &str, doc: Value) -> Result<bool, AppError> {
        let full_index = self.full_index_name(index);

        let response = self
            .client
            .index(IndexParts::IndexId(&full_index, id))
            .body(doc)
            .send()
            .await
            .map_err(|e| AppError::ElasticSearch(format!("Index error: {}", e)))?;

        Ok(response.status_code().is_success())
    }

    async fn bulk_index(&self, index: &str, docs: Vec<(String, Value)>) -> Result<Value, AppError> {
        let full_index = self.full_index_name(index);

        let mut body = Vec::new();
        for (id, doc) in docs {
            let action = json!({"index": {"_index": full_index, "_id": id}});
            body.push(action);
            body.push(doc);
        }

        let response = self
            .client
            .bulk(BulkParts::None)
            .body(body.into_iter().map(|v| v.to_string()).collect::<Vec<_>>())
            .send()
            .await
            .map_err(|e| AppError::ElasticSearch(format!("Bulk index error: {}", e)))?;

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

    async fn create_index(&self, index: &str, mapping: Value) -> Result<bool, AppError> {
        let full_index = self.full_index_name(index);

        let response = self
            .client
            .indices()
            .create(IndicesCreateParts::Index(&full_index))
            .body(mapping)
            .send()
            .await
            .map_err(|e| AppError::ElasticSearch(format!("Create index error: {}", e)))?;

        Ok(response.status_code().is_success())
    }

    async fn index_exists(&self, index: &str) -> Result<bool, AppError> {
        let full_index = self.full_index_name(index);

        let response = self
            .client
            .indices()
            .exists(IndicesExistsParts::Index(&[&full_index]))
            .send()
            .await
            .map_err(|e| AppError::ElasticSearch(format!("Index exists error: {}", e)))?;

        Ok(response.status_code().is_success())
    }
}
