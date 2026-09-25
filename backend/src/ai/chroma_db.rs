use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

pub struct ChromaClient {
    base_url: String,
    http: Client,
}

#[derive(Deserialize)]
struct CollectionResponse {
    id: String,
}

#[derive(Deserialize)]
struct QueryResponse {
    ids: Vec<Vec<String>>,
    distances: Option<Vec<Vec<f64>>>,
}

impl ChromaClient {
    // Dentro del bloque impl ChromaClient en backend/src/ai/chroma_db.rs:

    pub async fn add_or_update_ngo(
        &self,
        ngo_id: uuid::Uuid,
        needs_text: &str,
    ) -> Result<(), String> {
        // Si la colección 'ngos' requiere inserción vía HTTP API de ChromaDB:
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/collections", self.base_url);

        // 1. Obtener o crear colección
        let _col_res = client
            .post(&url)
            .json(&serde_json::json!({
                "name": "ngos",
                "metadata": { "hnsw:space": "cosine" }
            }))
            .send()
            .await;

        // 2. Insertar documento y embedding
        let add_url = format!("{}/api/v1/collections/ngos/upsert", self.base_url);
        let _ = client
            .post(&add_url)
            .json(&serde_json::json!({
                "ids": [ngo_id.to_string()],
                "documents": [needs_text],
                "metadatas": [{ "ngo_id": ngo_id.to_string() }]
            }))
            .send()
            .await;

        Ok(())
    }

    pub fn new(base_url: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:8001".to_string()),
            http: Client::new(),
        }
    }

    /// Obtiene o crea la colección en la API v2 de ChromaDB
    async fn get_or_create_collection(&self) -> Result<String, String> {
        // En Chroma v2 las colecciones residen bajo el tenant y base de datos por defecto
        let url = format!(
            "{}/api/v2/tenants/default_tenant/databases/default_database/collections",
            self.base_url
        );

        let resp = self
            .http
            .post(&url)
            .json(&json!({
                "name": "ngo_needs",
                "get_or_create": true
            }))
            .send()
            .await
            .map_err(|e| format!("Error conectando a ChromaDB: {}", e))?;

        let data = resp
            .json::<CollectionResponse>()
            .await
            .map_err(|e| format!("Error al deserializar colección: {}", e))?;

        Ok(data.id)
    }

    /// Indexa o actualiza la necesidad de una ONG
    pub async fn upsert_ngo_need(&self, ngo_id: Uuid, description: &str) -> Result<(), String> {
        let collection_id = self.get_or_create_collection().await?;
        let url = format!(
            "{}/api/v2/tenants/default_tenant/databases/default_database/collections/{}/add",
            self.base_url, collection_id
        );

        self.http
            .post(&url)
            .json(&json!({
                "ids": [ngo_id.to_string()],
                "documents": [description],
            }))
            .send()
            .await
            .map_err(|e| format!("Error indexando necesidad en ChromaDB: {}", e))?;

        Ok(())
    }

    /// Consulta semántica vectorial
    pub async fn query_similar_ngos(
        &self,
        query_text: &str,
        n_results: usize,
    ) -> Result<Vec<(Uuid, f64)>, String> {
        let collection_id = self.get_or_create_collection().await?;
        let url = format!(
            "{}/api/v2/tenants/default_tenant/databases/default_database/collections/{}/query",
            self.base_url, collection_id
        );

        let resp = self
            .http
            .post(&url)
            .json(&json!({
                "query_texts": [query_text],
                "n_results": n_results
            }))
            .send()
            .await
            .map_err(|e| format!("Error consultando ChromaDB: {}", e))?;

        let result = resp
            .json::<QueryResponse>()
            .await
            .map_err(|e| format!("Error leyendo respuesta vectorial: {}", e))?;

        let mut matches = Vec::new();
        if let (Some(ids), Some(distances)) = (
            result.ids.first(),
            result.distances.as_ref().and_then(|d| d.first()),
        ) {
            for (id_str, dist) in ids.iter().zip(distances.iter()) {
                if let Ok(uuid) = Uuid::parse_str(id_str) {
                    let similarity = (1.0 / (1.0 + dist)).clamp(0.0, 1.0);
                    matches.push((uuid, similarity));
                }
            }
        }

        Ok(matches)
    }
}
