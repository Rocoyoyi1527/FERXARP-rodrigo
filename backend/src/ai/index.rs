use sqlx::{FromRow, PgPool};
use tracing::{info, warn};
use uuid::Uuid;

use super::chroma_db::{COLLECTION, ChromaClient, ChromaError};

#[derive(Debug, Clone, FromRow)]
pub struct NgoIndexRecord {
    pub id: Uuid,
    pub name: String,
    pub needs_description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReindexReport {
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
}

pub fn build_ngo_semantic_document(ngo: &NgoIndexRecord) -> String {
    match ngo
        .needs_description
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        Some(needs) => format!("{}: {}", ngo.name.trim(), needs),
        None => ngo.name.trim().to_string(),
    }
}

pub async fn upsert_ngo(chroma: &ChromaClient, ngo: &NgoIndexRecord) -> Result<(), ChromaError> {
    chroma
        .upsert_ngo(ngo.id, &ngo.name, &build_ngo_semantic_document(ngo))
        .await?;
    info!(ngo_id = %ngo.id, collection = COLLECTION, "chroma upsert success");
    Ok(())
}

pub async fn reindex_all(
    pool: &PgPool,
    chroma: &ChromaClient,
) -> Result<ReindexReport, &'static str> {
    let ngos = sqlx::query_as::<_, NgoIndexRecord>(
        "SELECT id, name, needs_description FROM ngos ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| "No se pudieron leer las ONG de PostgreSQL")?;

    chroma
        .health()
        .await
        .map_err(|_| "ChromaDB no está disponible")?;
    match chroma.get_or_create_collection().await {
        Ok(_) => {}
        Err(ChromaError::IncompatibleMetric) => {
            warn!(
                collection = COLLECTION,
                "replacing legacy Chroma collection with incompatible metric"
            );
            chroma
                .delete_collection_by_name()
                .await
                .map_err(|_| "No se pudo reemplazar el índice ChromaDB anterior")?;
            chroma
                .get_or_create_collection()
                .await
                .map_err(|_| "No se pudo preparar la colección ChromaDB")?;
        }
        Err(_) => return Err("No se pudo preparar la colección ChromaDB"),
    }

    let mut report = ReindexReport {
        processed: ngos.len(),
        succeeded: 0,
        failed: 0,
    };
    for ngo in &ngos {
        match upsert_ngo(chroma, ngo).await {
            Ok(()) => report.succeeded += 1,
            Err(error) => {
                report.failed += 1;
                warn!(ngo_id = %ngo.id, error = %error, "chroma reindex failed");
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_document_is_deterministic() {
        let ngo = NgoIndexRecord {
            id: Uuid::nil(),
            name: " Banco de Alimentos ".into(),
            needs_description: Some(" leche y arroz ".into()),
        };
        assert_eq!(
            build_ngo_semantic_document(&ngo),
            "Banco de Alimentos: leche y arroz"
        );
        assert_eq!(
            build_ngo_semantic_document(&ngo),
            build_ngo_semantic_document(&ngo)
        );
    }
}
