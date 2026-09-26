use std::{env, str::FromStr};

use backend::ai::{
    chroma_db::{ChromaClient, ChromaError},
    index::{NgoIndexRecord, build_ngo_semantic_document, reindex_all, upsert_ngo},
};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use uuid::Uuid;

fn local_chroma_url() -> String {
    dotenv::dotenv().ok();
    let url =
        env::var("CHROMA_URL").expect("CHROMA_URL local required for Chroma integration tests");
    let parsed = reqwest::Url::parse(&url).expect("valid CHROMA_URL");
    assert!(
        matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "::1")),
        "Chroma integration tests require loopback ChromaDB"
    );
    url
}

fn local_chroma() -> ChromaClient {
    ChromaClient::with_collection(
        &local_chroma_url(),
        &format!("test_{}", Uuid::new_v4().simple()),
    )
    .unwrap()
}

fn ngo(name: &str, needs: &str) -> NgoIndexRecord {
    NgoIndexRecord {
        id: Uuid::new_v4(),
        name: name.into(),
        needs_description: Some(needs.into()),
    }
}

async fn local_test_pool() -> PgPool {
    dotenv::dotenv().ok();
    let url = env::var("DATABASE_URL").expect("DATABASE_URL local required");
    let options = PgConnectOptions::from_str(&url).expect("valid local DATABASE_URL");
    assert!(
        matches!(options.get_host(), "localhost" | "127.0.0.1" | "::1"),
        "Chroma integration tests require loopback PostgreSQL"
    );
    let bootstrap = PgPoolOptions::new()
        .connect_with(options.clone())
        .await
        .unwrap();
    if let Err(error) = sqlx::query("CREATE DATABASE ferxarp_chroma_test")
        .execute(&bootstrap)
        .await
    {
        assert_eq!(
            error
                .as_database_error()
                .and_then(|db| db.code())
                .as_deref(),
            Some("42P04")
        );
    }
    bootstrap.close().await;
    let pool = PgPoolOptions::new()
        .connect_with(options.database("ferxarp_chroma_test"))
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn chroma_01_collection_get_or_create() {
    let chroma = local_chroma();
    chroma.health().await.unwrap();
    let first = chroma.get_or_create_collection().await.unwrap();
    assert_eq!(first, chroma.get_or_create_collection().await.unwrap());
    chroma.delete_collection().await.unwrap();
}

#[tokio::test]
async fn chroma_02_upsert_and_query() {
    let chroma = local_chroma();
    let milk = ngo("Banco", "leche y lácteos para familias");
    upsert_ngo(&chroma, &milk).await.unwrap();
    let result = chroma.query_similar_ngos("leche", 2).await.unwrap();
    assert_eq!(result[0].0, milk.id);
    assert!(result[0].1 > 0.0);
    assert_eq!(
        chroma.get_document(milk.id).await.unwrap(),
        Some(build_ngo_semantic_document(&milk))
    );
    chroma.delete_collection().await.unwrap();
}

#[tokio::test]
async fn chroma_03_repeated_upsert_has_one_id() {
    let chroma = local_chroma();
    let milk = ngo("Banco", "leche");
    upsert_ngo(&chroma, &milk).await.unwrap();
    upsert_ngo(&chroma, &milk).await.unwrap();
    assert_eq!(chroma.count().await.unwrap(), 1);
    chroma.delete_collection().await.unwrap();
}

#[tokio::test]
async fn chroma_04_modified_needs_replace_document() {
    let chroma = local_chroma();
    let mut item = ngo("Banco", "leche");
    upsert_ngo(&chroma, &item).await.unwrap();
    item.needs_description = Some("computadoras y laptops".into());
    upsert_ngo(&chroma, &item).await.unwrap();
    assert_eq!(chroma.count().await.unwrap(), 1);
    assert_eq!(
        chroma.get_document(item.id).await.unwrap(),
        Some(build_ngo_semantic_document(&item))
    );
    chroma.delete_collection().await.unwrap();
}

#[tokio::test]
async fn chroma_05_relevant_ngo_ranks_first() {
    let chroma = local_chroma();
    let milk = ngo("Banco", "leche lácteos alimento");
    let computers = ngo("Escuela", "computadoras laptops servidores");
    upsert_ngo(&chroma, &milk).await.unwrap();
    upsert_ngo(&chroma, &computers).await.unwrap();
    let result = chroma
        .query_similar_ngos("donación de leche", 2)
        .await
        .unwrap();
    assert_eq!(result[0].0, milk.id);
    assert!(result[0].1 > result[1].1);
    chroma.delete_collection().await.unwrap();
}

#[tokio::test]
async fn chroma_06_reindex_rebuilds_from_postgres() {
    let pool = local_test_pool().await;
    let chroma = local_chroma();
    for (email, name, needs) in [
        (
            "chroma-milk@example.test",
            "Banco de Alimentos",
            "leche y arroz",
        ),
        (
            "chroma-computers@example.test",
            "Centro de Cómputo",
            "laptops y computadoras",
        ),
    ] {
        let user_id: Uuid = sqlx::query_scalar("INSERT INTO users (email, password_hash, role) VALUES ($1, 'test-hash', 'ong') ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email RETURNING id")
            .bind(email).fetch_one(&pool).await.unwrap();
        sqlx::query("INSERT INTO ngos (user_id, name, needs_description) VALUES ($1, $2, $3) ON CONFLICT (user_id) DO UPDATE SET name = EXCLUDED.name, needs_description = EXCLUDED.needs_description")
            .bind(user_id).bind(name).bind(needs).execute(&pool).await.unwrap();
    }
    let first = reindex_all(&pool, &chroma).await.unwrap();
    assert_eq!(first.failed, 0);
    assert!(first.processed >= 2);
    assert_eq!(chroma.count().await.unwrap(), first.processed);
    let second = reindex_all(&pool, &chroma).await.unwrap();
    assert_eq!(chroma.count().await.unwrap(), second.processed);
    chroma.delete_collection().await.unwrap();
    pool.close().await;
}

#[tokio::test]
async fn chroma_07_missing_collection_is_created() {
    let chroma = local_chroma();
    let record = ngo("Banco", "leche");
    upsert_ngo(&chroma, &record).await.unwrap();
    assert_eq!(chroma.count().await.unwrap(), 1);
    chroma.delete_collection().await.unwrap();
}

#[tokio::test]
async fn chroma_08_unavailable_is_error_not_empty_matches() {
    let chroma = ChromaClient::new("http://127.0.0.1:1").unwrap();
    assert_eq!(
        chroma.query_similar_ngos("leche", 2).await.unwrap_err(),
        ChromaError::Unavailable
    );
}

#[tokio::test]
async fn chroma_09_empty_collection_is_not_an_error() {
    let chroma = local_chroma();
    chroma.get_or_create_collection().await.unwrap();
    assert!(
        chroma
            .query_similar_ngos("leche", 1)
            .await
            .unwrap()
            .is_empty()
    );
    chroma.delete_collection().await.unwrap();
}

#[tokio::test]
async fn chroma_10_query_filters_to_postgres_candidate_ids() {
    let chroma = local_chroma();
    let milk = ngo("Banco", "leche lácteos");
    let other = ngo("Otro banco", "leche lácteos");
    upsert_ngo(&chroma, &milk).await.unwrap();
    upsert_ngo(&chroma, &other).await.unwrap();
    let result = chroma
        .query_similar_ngos_for("leche", 1, &[milk.id])
        .await
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, milk.id);
    chroma.delete_collection().await.unwrap();
}

#[tokio::test]
async fn chroma_11_reindex_replaces_legacy_metric() {
    let url = local_chroma_url();
    let name = format!("test_{}", Uuid::new_v4().simple());
    let response = reqwest::Client::new()
        .post(format!(
            "{url}/api/v2/tenants/default_tenant/databases/default_database/collections"
        ))
        .json(&serde_json::json!({ "name": name, "get_or_create": true }))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let chroma = ChromaClient::with_collection(&url, &name).unwrap();
    assert_eq!(
        chroma.get_or_create_collection().await.unwrap_err(),
        ChromaError::IncompatibleMetric
    );
    let pool = local_test_pool().await;
    let report = reindex_all(&pool, &chroma).await.unwrap();
    assert_eq!(report.failed, 0);
    assert_eq!(chroma.count().await.unwrap(), report.processed);
    chroma.delete_collection().await.unwrap();
    pool.close().await;
}
