use std::{
    env,
    time::{Duration, Instant},
};

use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

use super::embedding::embed;

pub const COLLECTION: &str = "ngo_needs";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaError {
    MissingConfiguration,
    Unavailable,
    Timeout,
    Api,
    IncompatibleMetric,
    IncompatibleResponse,
}

impl std::fmt::Display for ChromaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::MissingConfiguration => "CHROMA_URL no está configurada",
            Self::Unavailable => "ChromaDB no está disponible",
            Self::Timeout => "ChromaDB excedió el tiempo de espera",
            Self::Api => "ChromaDB rechazó la operación",
            Self::IncompatibleMetric => "La colección ChromaDB usa una métrica incompatible",
            Self::IncompatibleResponse => "Respuesta incompatible de ChromaDB",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ChromaError {}

fn request_error(error: reqwest::Error) -> ChromaError {
    if error.is_timeout() {
        ChromaError::Timeout
    } else {
        ChromaError::Unavailable
    }
}

#[derive(Clone)]
pub struct ChromaClient {
    base_url: String,
    collection: String,
    http: Client,
}

#[derive(Deserialize)]
struct CollectionResponse {
    id: String,
    configuration_json: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct QueryResponse {
    ids: Vec<Vec<String>>,
    distances: Option<Vec<Vec<f64>>>,
}

#[derive(Deserialize)]
struct GetResponse {
    ids: Vec<String>,
    documents: Option<Vec<Option<String>>>,
}

impl ChromaClient {
    pub fn from_env() -> Result<Self, ChromaError> {
        let url = env::var("CHROMA_URL").map_err(|_| ChromaError::MissingConfiguration)?;
        Self::new(&url)
    }

    pub fn new(base_url: &str) -> Result<Self, ChromaError> {
        Self::with_collection(base_url, COLLECTION)
    }

    pub fn with_collection(base_url: &str, collection: &str) -> Result<Self, ChromaError> {
        let url = reqwest::Url::parse(base_url).map_err(|_| ChromaError::MissingConfiguration)?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || collection.is_empty()
        {
            return Err(ChromaError::MissingConfiguration);
        }
        let http = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| ChromaError::MissingConfiguration)?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            collection: collection.to_string(),
            http,
        })
    }

    fn collections_url(&self) -> String {
        format!(
            "{}/api/v2/tenants/default_tenant/databases/default_database/collections",
            self.base_url
        )
    }

    fn item_url(&self, collection_id: &str, operation: &str) -> String {
        format!("{}/{collection_id}/{operation}", self.collections_url())
    }

    pub async fn health(&self) -> Result<(), ChromaError> {
        self.http
            .get(format!("{}/api/v2/heartbeat", self.base_url))
            .send()
            .await
            .map_err(request_error)?
            .error_for_status()
            .map_err(|_| ChromaError::Api)?;
        Ok(())
    }

    pub async fn get_or_create_collection(&self) -> Result<String, ChromaError> {
        let response = self.http.post(self.collections_url())
            .json(&json!({ "name": self.collection, "get_or_create": true, "metadata": { "hnsw:space": "cosine" } }))
            .send().await.map_err(request_error)?
            .error_for_status().map_err(|_| ChromaError::Api)?;
        let collection = response
            .json::<CollectionResponse>()
            .await
            .map_err(|_| ChromaError::IncompatibleResponse)?;
        if collection.id.is_empty() {
            return Err(ChromaError::IncompatibleResponse);
        }
        let metric = collection
            .configuration_json
            .as_ref()
            .and_then(|config| config.pointer("/hnsw/space"))
            .or_else(|| {
                collection
                    .metadata
                    .as_ref()
                    .and_then(|metadata| metadata.get("hnsw:space"))
            })
            .and_then(serde_json::Value::as_str);
        if metric != Some("cosine") {
            return Err(ChromaError::IncompatibleMetric);
        }
        Ok(collection.id)
    }

    pub async fn upsert_ngo(
        &self,
        ngo_id: Uuid,
        name: &str,
        document: &str,
    ) -> Result<(), ChromaError> {
        let collection_id = self.get_or_create_collection().await?;
        self.http
            .post(self.item_url(&collection_id, "upsert"))
            .json(&json!({
                "ids": [ngo_id.to_string()],
                "documents": [document],
                "metadatas": [{ "ngo_id": ngo_id.to_string(), "nombre": name }],
                "embeddings": [embed(document)]
            }))
            .send()
            .await
            .map_err(request_error)?
            .error_for_status()
            .map_err(|_| ChromaError::Api)?;
        Ok(())
    }

    pub async fn query_similar_ngos(
        &self,
        query_text: &str,
        n_results: usize,
    ) -> Result<Vec<(Uuid, f64)>, ChromaError> {
        self.query_similar_ngos_for(query_text, n_results, &[])
            .await
    }

    pub async fn query_similar_ngos_for(
        &self,
        query_text: &str,
        n_results: usize,
        candidate_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, f64)>, ChromaError> {
        let start = Instant::now();
        let collection_id = self.get_or_create_collection().await?;
        let mut payload = json!({ "query_embeddings": [embed(query_text)], "n_results": n_results, "include": ["distances"] });
        if !candidate_ids.is_empty() {
            let ids: Vec<String> = candidate_ids.iter().map(Uuid::to_string).collect();
            payload["where"] = json!({ "ngo_id": { "$in": ids } });
        }
        let response = self
            .http
            .post(self.item_url(&collection_id, "query"))
            .json(&payload)
            .send()
            .await
            .map_err(request_error)?
            .error_for_status()
            .map_err(|_| ChromaError::Api)?;
        let result = response
            .json::<QueryResponse>()
            .await
            .map_err(|_| ChromaError::IncompatibleResponse)?;
        let ids = result
            .ids
            .first()
            .ok_or(ChromaError::IncompatibleResponse)?;
        let distances = result
            .distances
            .as_ref()
            .and_then(|rows| rows.first())
            .ok_or(ChromaError::IncompatibleResponse)?;
        if ids.len() != distances.len() {
            return Err(ChromaError::IncompatibleResponse);
        }
        let matches = ids
            .iter()
            .zip(distances)
            .map(|(id, distance)| {
                let id = Uuid::parse_str(id).map_err(|_| ChromaError::IncompatibleResponse)?;
                if !distance.is_finite() {
                    return Err(ChromaError::IncompatibleResponse);
                }
                Ok((id, cosine_distance_score(*distance)))
            })
            .collect::<Result<Vec<_>, ChromaError>>()?;
        info!(collection = %self.collection, result_count = matches.len(), duration_ms = start.elapsed().as_millis(), "chroma query success");
        Ok(matches)
    }

    pub async fn get_document(&self, ngo_id: Uuid) -> Result<Option<String>, ChromaError> {
        let collection_id = self.get_or_create_collection().await?;
        let response = self
            .http
            .post(self.item_url(&collection_id, "get"))
            .json(&json!({ "ids": [ngo_id.to_string()], "include": ["documents"] }))
            .send()
            .await
            .map_err(request_error)?
            .error_for_status()
            .map_err(|_| ChromaError::Api)?;
        let result = response
            .json::<GetResponse>()
            .await
            .map_err(|_| ChromaError::IncompatibleResponse)?;
        if result.ids.is_empty() {
            return Ok(None);
        }
        Ok(result
            .documents
            .and_then(|docs| docs.into_iter().next().flatten()))
    }

    pub async fn count(&self) -> Result<usize, ChromaError> {
        let collection_id = self.get_or_create_collection().await?;
        let response = self
            .http
            .get(self.item_url(&collection_id, "count"))
            .send()
            .await
            .map_err(request_error)?
            .error_for_status()
            .map_err(|_| ChromaError::Api)?;
        response
            .json::<usize>()
            .await
            .map_err(|_| ChromaError::IncompatibleResponse)
    }

    pub async fn delete_collection(&self) -> Result<(), ChromaError> {
        self.get_or_create_collection().await?;
        self.delete_collection_by_name().await
    }

    /// Only the explicit reindex operation uses this to replace a legacy derived collection.
    pub async fn delete_collection_by_name(&self) -> Result<(), ChromaError> {
        self.http
            .delete(format!("{}/{}", self.collections_url(), self.collection))
            .send()
            .await
            .map_err(request_error)?
            .error_for_status()
            .map_err(|_| ChromaError::Api)?;
        warn!(collection = %self.collection, "chroma collection deleted");
        Ok(())
    }
}

/// Cosine distance is 1 - cosine similarity. Negative similarity contributes zero.
pub fn cosine_distance_score(distance: f64) -> f64 {
    (1.0 - distance).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_score_is_bounded() {
        assert_eq!(cosine_distance_score(0.0), 1.0);
        assert_eq!(cosine_distance_score(1.0), 0.0);
        assert_eq!(cosine_distance_score(2.0), 0.0);
    }
}
