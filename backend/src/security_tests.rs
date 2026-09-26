use std::{str::FromStr, sync::Arc};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde_json::{Value, json};
use sqlx::{
    AssertSqlSafe, PgPool, Row,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::sync::OnceCell;
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    AppState,
    ai::{
        chroma_db::ChromaClient,
        index::{NgoIndexRecord, upsert_ngo},
    },
    api::auth::Claims,
    models::user::Role,
};
use backend::admin_bootstrap::{
    ProvisionConfig, ProvisionError, ProvisionOutcome, provision_admin,
};

const SECRET: &str = "local-test-jwt-secret-only";
static DB_READY: OnceCell<()> = OnceCell::const_new();

fn local_options() -> PgConnectOptions {
    dotenv::dotenv().ok();
    let url =
        std::env::var("DATABASE_URL").expect("Local DATABASE_URL required for security tests");
    let options = PgConnectOptions::from_str(&url).expect("Invalid local DATABASE_URL");
    assert!(
        matches!(options.get_host(), "localhost" | "127.0.0.1" | "::1"),
        "Security tests require loopback PostgreSQL"
    );
    assert_ne!(
        options.get_database(),
        Some("ferxarp_security_test"),
        "Security tests require a separate local bootstrap database"
    );
    options
}

async fn pool() -> PgPool {
    DB_READY
        .get_or_init(|| async {
            let options = local_options();
            let bootstrap = PgPoolOptions::new()
                .connect_with(options.clone())
                .await
                .expect("Connect to local PostgreSQL");
            let create = sqlx::query("CREATE DATABASE ferxarp_security_test")
                .execute(&bootstrap)
                .await;
            if let Err(error) = create {
                assert_eq!(
                    error
                        .as_database_error()
                        .and_then(|db| db.code())
                        .as_deref(),
                    Some("42P04"),
                    "Create isolated security test database: {error}"
                );
            }
            bootstrap.close().await;
            let pool = PgPoolOptions::new()
                .max_connections(3)
                .connect_with(options.database("ferxarp_security_test"))
                .await
                .expect("Connect to test database");
            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .expect("Migrate test database");
            pool.close().await;
        })
        .await;
    PgPoolOptions::new()
        .max_connections(3)
        .connect_with(local_options().database("ferxarp_security_test"))
        .await
        .expect("Connect to isolated security test database")
}

async fn app() -> (Router, PgPool) {
    let pool = pool().await;
    (router_for(&pool), pool)
}

fn router_for(pool: &PgPool) -> Router {
    router_for_with_chroma(pool, None)
}

fn router_for_with_chroma(pool: &PgPool, chroma_url: Option<String>) -> Router {
    router_for_with_config(pool, chroma_url, None)
}

fn router_for_with_config(
    pool: &PgPool,
    chroma_url: Option<String>,
    seed_password: Option<String>,
) -> Router {
    super::build_router(Arc::new(AppState {
        db: pool.clone(),
        jwt_secret: SECRET.to_string(),
        seed_password,
        chroma_url,
    }))
}

#[tokio::test]
async fn cors_allows_only_configured_frontend_and_sets_nosniff() {
    let app = router_for(&pool().await);
    let allowed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("origin", "http://localhost:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::OK);
    assert_eq!(
        allowed
            .headers()
            .get("access-control-allow-origin")
            .unwrap(),
        "http://localhost:3000"
    );
    assert_eq!(
        allowed.headers().get("x-content-type-options").unwrap(),
        "nosniff"
    );

    let denied = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("origin", "https://untrusted.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::OK);
    assert!(
        denied
            .headers()
            .get("access-control-allow-origin")
            .is_none()
    );
}

#[tokio::test]
async fn seed_veracruz_reuses_chroma_index_without_groq() {
    let pool = pool().await;
    dotenv::dotenv().ok();
    let url = std::env::var("CHROMA_URL").expect("Local CHROMA_URL required");
    let parsed = reqwest::Url::parse(&url).unwrap();
    assert!(matches!(
        parsed.host_str(),
        Some("localhost" | "127.0.0.1" | "::1")
    ));
    let chroma = ChromaClient::new(&url).unwrap();
    let app = router_for_with_config(&pool, Some(url), Some(Uuid::new_v4().to_string()));
    let admin = user(&pool, Role::Admin).await;
    let admin_token = token(admin, Role::Admin);

    for _ in 0..2 {
        let (status, body) = send(
            &app,
            "POST",
            "/api/seed/veracruz?include_ai=false",
            Some(&admin_token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["ngos_seeded"], 5);
        assert_eq!(body["ai_evaluations"].as_array().unwrap().len(), 0);
    }
    let bank_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM ngos WHERE name = 'Banco de Alimentos de Veracruz (AMBA)'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let document = chroma.get_document(bank_id).await.unwrap().unwrap();
    assert!(document.contains("leche"));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM ngos WHERE id = $1")
        .bind(bank_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn new_ngo_registration_indexes_its_postgres_profile() {
    let pool = pool().await;
    dotenv::dotenv().ok();
    let url = std::env::var("CHROMA_URL").expect("Local CHROMA_URL required");
    let parsed = reqwest::Url::parse(&url).unwrap();
    assert!(matches!(
        parsed.host_str(),
        Some("localhost" | "127.0.0.1" | "::1")
    ));
    let chroma = ChromaClient::new(&url).unwrap();
    let app = router_for_with_chroma(&pool, Some(url));
    let email = format!("{}@test.local", Uuid::new_v4());
    let (status, _) = send(
        &app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({
            "email": email,
            "password": Uuid::new_v4().to_string(),
            "role": "ong"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let ngo_id: Uuid = sqlx::query_scalar(
        "SELECT n.id FROM ngos n JOIN users u ON n.user_id = u.id WHERE u.email = $1",
    )
    .bind(email)
    .fetch_one(&pool)
    .await
    .unwrap();
    let document = chroma.get_document(ngo_id).await.unwrap().unwrap();
    assert!(document.contains("Recepción y distribución comunitaria"));
}

async fn matching_response(
    app: &Router,
    donation_id: Uuid,
    owner: Uuid,
) -> (StatusCode, String, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/donations/{donation_id}/matches?include_ai=false"
                ))
                .header(
                    "authorization",
                    format!("Bearer {}", token(owner, Role::Empresa)),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let mode = response
        .headers()
        .get("x-matching-mode")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, mode, serde_json::from_slice(&body).unwrap())
}

async fn matching_scenario(pool: &PgPool) -> (Uuid, Uuid, Uuid, NgoIndexRecord, NgoIndexRecord) {
    let owner = user(pool, Role::Empresa).await;
    let milk_user = user(pool, Role::Ong).await;
    let computer_user = user(pool, Role::Ong).await;
    let milk_id = ngo(pool, milk_user, true).await;
    let computer_id = ngo(pool, computer_user, true).await;
    let milk = NgoIndexRecord {
        id: milk_id,
        name: "Banco de Alimentos".into(),
        needs_description: Some("leche y lácteos".into()),
    };
    let computers = NgoIndexRecord {
        id: computer_id,
        name: "Centro de Cómputo".into(),
        needs_description: Some("computadoras y laptops".into()),
    };
    for record in [&milk, &computers] {
        sqlx::query("UPDATE ngos SET name = $1, needs_description = $2, latitude = 19.1738, longitude = -96.1342 WHERE id = $3")
            .bind(&record.name).bind(&record.needs_description).bind(record.id).execute(pool).await.unwrap();
    }
    let donation_id: Uuid = sqlx::query_scalar("INSERT INTO donations (user_id, title, description, quantity) VALUES ($1, 'Leche', 'lácteos', 2) RETURNING id")
        .bind(owner).fetch_one(pool).await.unwrap();
    (owner, donation_id, milk_id, milk, computers)
}

#[tokio::test]
async fn matching_endpoint_hybrid_without_groq() {
    let pool = pool().await;
    dotenv::dotenv().ok();
    let url = std::env::var("CHROMA_URL").expect("Local CHROMA_URL required");
    let parsed = reqwest::Url::parse(&url).unwrap();
    assert!(matches!(
        parsed.host_str(),
        Some("localhost" | "127.0.0.1" | "::1")
    ));
    let chroma = ChromaClient::new(&url).unwrap();
    let (owner, donation_id, milk_id, milk, computers) = matching_scenario(&pool).await;
    upsert_ngo(&chroma, &milk).await.unwrap();
    upsert_ngo(&chroma, &computers).await.unwrap();
    let (status, mode, body) = matching_response(
        &router_for_with_chroma(&pool, Some(url)),
        donation_id,
        owner,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mode, "hybrid");
    let rows = body.as_array().unwrap();
    let milk_row = rows
        .iter()
        .find(|row| row["ngo_id"] == milk_id.to_string())
        .unwrap();
    assert!(milk_row["lexical_score"].as_f64().unwrap() > 0.0);
    assert!(milk_row["vector_score"].as_f64().unwrap() > 0.0);
    assert!(milk_row["ai_reasoning"].is_null());
}

#[tokio::test]
async fn matching_endpoint_reports_lexical_fallback_when_chroma_is_down() {
    let pool = pool().await;
    let (owner, donation_id, milk_id, _, _) = matching_scenario(&pool).await;
    let app = router_for_with_chroma(&pool, Some("http://127.0.0.1:1".into()));
    let (status, mode, body) = matching_response(&app, donation_id, owner).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mode, "lexical_fallback");
    let milk_row = body
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["ngo_id"] == milk_id.to_string())
        .unwrap();
    assert!(milk_row["lexical_score"].as_f64().unwrap() > 0.0);
    assert_eq!(milk_row["vector_score"].as_f64().unwrap(), 0.0);
}

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let bytes = body.map(|value| value.to_string()).unwrap_or_default();
    if !bytes.is_empty() {
        builder = builder.header("content-type", "application/json");
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(bytes)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&body)
        .unwrap_or_else(|_| json!({ "text": String::from_utf8_lossy(&body) }));
    (status, value)
}

fn token(user_id: Uuid, role: Role) -> String {
    token_exp(
        user_id,
        role,
        (chrono::Utc::now().timestamp() + 3600) as usize,
    )
}

fn token_exp(user_id: Uuid, role: Role, exp: usize) -> String {
    encode(
        &Header::default(),
        &Claims {
            sub: user_id,
            role,
            exp,
        },
        &EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .unwrap()
}

async fn user(pool: &PgPool, role: Role) -> Uuid {
    let role_name = match role {
        Role::Admin => "admin",
        Role::Empresa => "empresa",
        Role::Ong => "ong",
        Role::Ceo => "ceo",
    };
    sqlx::query("INSERT INTO users (email, password_hash, role) VALUES ($1, 'test-hash', $2::user_role) RETURNING id")
        .bind(format!("{}@test.local", Uuid::new_v4())).bind(role_name).fetch_one(pool).await.unwrap().get("id")
}

async fn ngo(pool: &PgPool, user_id: Uuid, verified: bool) -> Uuid {
    sqlx::query(
        "INSERT INTO ngos (user_id, name, is_verified) VALUES ($1, 'Test NGO', $2) RETURNING id",
    )
    .bind(user_id)
    .bind(verified)
    .fetch_one(pool)
    .await
    .unwrap()
    .get("id")
}

async fn donation(pool: &PgPool, owner: Uuid, assigned: Option<Uuid>) -> Uuid {
    sqlx::query("INSERT INTO donations (user_id, title, quantity, assigned_ngo_id) VALUES ($1, 'Test donation', 1, $2) RETURNING id")
        .bind(owner).bind(assigned).fetch_one(pool).await.unwrap().get("id")
}

async fn request(pool: &PgPool, donation_id: Uuid, ngo_id: Uuid) {
    sqlx::query("INSERT INTO donation_requests (donation_id, ngo_id) VALUES ($1, $2)")
        .bind(donation_id)
        .bind(ngo_id)
        .execute(pool)
        .await
        .unwrap();
}

async fn reserved_donation(pool: &PgPool, owner: Uuid, ngo_id: Uuid) -> Uuid {
    let id = donation(pool, owner, None).await;
    request(pool, id, ngo_id).await;
    sqlx::query("UPDATE donations SET status = 'reservado' WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn approved_reserved_donation(pool: &PgPool, owner: Uuid, ngo_id: Uuid) -> Uuid {
    let id = reserved_donation(pool, owner, ngo_id).await;
    sqlx::query("UPDATE donation_requests SET status = 'aprobada' WHERE donation_id = $1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE donations SET assigned_ngo_id = $1 WHERE id = $2")
        .bind(ngo_id)
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn in_transit_donation(pool: &PgPool, owner: Uuid, ngo_id: Uuid) -> Uuid {
    let id = approved_reserved_donation(pool, owner, ngo_id).await;
    sqlx::query("UPDATE donations SET status = 'en_transito' WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn donation_state(pool: &PgPool, id: Uuid) -> (String, Option<Uuid>, Option<String>, bool) {
    sqlx::query_as(
        "SELECT status, assigned_ngo_id, rejection_reason, completed_at IS NOT NULL FROM donations WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn log_count(pool: &PgPool, id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM delivery_logs WHERE donation_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn scan(
    app: &Router,
    id: Uuid,
    user_id: Uuid,
    role: Role,
    action: &str,
    reason: Option<&str>,
) -> StatusCode {
    send(
        app,
        "POST",
        "/api/scanner/scan",
        Some(&token(user_id, role)),
        Some(json!({
            "donation_id": id, "action": action, "rejection_reason": reason
        })),
    )
    .await
    .0
}

#[tokio::test]
async fn auth_01_empresa_registration_allowed() {
    let (app, _) = app().await;
    let (status, _) = send(&app, "POST", "/api/auth/register", None, Some(json!({"email": format!("{}@test.local", Uuid::new_v4()), "password": "test-password", "role": "empresa"}))).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn auth_02_ong_registration_creates_unverified_profile() {
    let (app, pool) = app().await;
    let email = format!("{}@test.local", Uuid::new_v4());
    let (status, _) = send(
        &app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({"email": email, "password": "test-password", "role": "ong"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let verified: bool = sqlx::query(
        "SELECT n.is_verified FROM ngos n JOIN users u ON u.id = n.user_id WHERE u.email = $1",
    )
    .bind(email)
    .fetch_one(&pool)
    .await
    .unwrap()
    .get("is_verified");
    assert!(!verified);
}

#[tokio::test]
async fn auth_03_04_privileged_registration_rejected() {
    let (app, pool) = app().await;
    for role in ["admin", "ceo"] {
        let email = format!("{}@test.local", Uuid::new_v4());
        let (status, _) = send(
            &app,
            "POST",
            "/api/auth/register",
            None,
            Some(json!({"email": email, "password": "test-password", "role": role})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "role {role}");
        let count: i64 = sqlx::query("SELECT count(*) AS count FROM users WHERE email = $1")
            .bind(email)
            .fetch_one(&pool)
            .await
            .unwrap()
            .get("count");
        assert_eq!(count, 0);
    }
}

#[tokio::test]
async fn auth_05_06_missing_invalid_and_expired_jwt_rejected() {
    let (app, _) = app().await;
    for token in [None, Some("invalid"), Some(""), Some("garbage")] {
        let (status, _) = send(&app, "GET", "/api/auth/me", token, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let expired = token_exp(Uuid::new_v4(), Role::Empresa, 1);
    assert_eq!(
        send(&app, "GET", "/api/auth/me", Some(&expired), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn ong_01_unverified_cannot_request() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ong_user = user(&pool, Role::Ong).await;
    let _ = ngo(&pool, ong_user, false).await;
    let donation = donation(&pool, owner, None).await;
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{donation}/request"),
            Some(&token(ong_user, Role::Ong)),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn ong_missing_profile_cannot_request() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ong_user = user(&pool, Role::Ong).await;
    let donation = donation(&pool, owner, None).await;
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{donation}/request"),
            Some(&token(ong_user, Role::Ong)),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn ong_02_verified_can_request() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ong_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ong_user, true).await;
    let donation = donation(&pool, owner, None).await;
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{donation}/request"),
            Some(&token(ong_user, Role::Ong)),
            None
        )
        .await
        .0,
        StatusCode::CREATED
    );
    let actual: Uuid = sqlx::query("SELECT ngo_id FROM donation_requests WHERE donation_id = $1")
        .bind(donation)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("ngo_id");
    assert_eq!(actual, ngo_id);
}

#[tokio::test]
async fn ong_03_cannot_impersonate_another_ngo() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let a = user(&pool, Role::Ong).await;
    let b = user(&pool, Role::Ong).await;
    let a_ngo = ngo(&pool, a, true).await;
    let b_ngo = ngo(&pool, b, true).await;
    let donation = donation(&pool, owner, None).await;
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{donation}/request"),
            Some(&token(a, Role::Ong)),
            Some(json!({"ngo_id": b_ngo}))
        )
        .await
        .0,
        StatusCode::CREATED
    );
    let actual: Uuid = sqlx::query("SELECT ngo_id FROM donation_requests WHERE donation_id = $1")
        .bind(donation)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("ngo_id");
    assert_eq!(actual, a_ngo);
}

#[tokio::test]
async fn ong_wrong_roles_cannot_request() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let donation = donation(&pool, owner, None).await;
    for role in [Role::Empresa, Role::Admin, Role::Ceo] {
        let id = user(&pool, role.clone()).await;
        assert_eq!(
            send(
                &app,
                "POST",
                &format!("/api/donations/{donation}/request"),
                Some(&token(id, role)),
                None
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
}

#[tokio::test]
async fn ong_missing_donation_returns_404() {
    let (app, pool) = app().await;
    let ong_user = user(&pool, Role::Ong).await;
    let _ = ngo(&pool, ong_user, true).await;
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{}/request", Uuid::new_v4()),
            Some(&token(ong_user, Role::Ong)),
            None,
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn don_01_owner_can_approve_request() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ong_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ong_user, true).await;
    let donation = reserved_donation(&pool, owner, ngo_id).await;
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{donation}/approve"),
            Some(&token(owner, Role::Empresa)),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let status: String = sqlx::query("SELECT status FROM donation_requests WHERE donation_id = $1")
        .bind(donation)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("status");
    assert_eq!(status, "aprobada");
    let donation_status: String = sqlx::query_scalar("SELECT status FROM donations WHERE id = $1")
        .bind(donation)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(donation_status, "reservado");
}

#[tokio::test]
async fn don_approval_updates_only_assigned_ngo_request() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let assigned_user = user(&pool, Role::Ong).await;
    let other_user = user(&pool, Role::Ong).await;
    let assigned = ngo(&pool, assigned_user, true).await;
    let other = ngo(&pool, other_user, true).await;
    let donation = reserved_donation(&pool, owner, assigned).await;
    let second = sqlx::query("INSERT INTO donation_requests (donation_id, ngo_id) VALUES ($1, $2)")
        .bind(donation)
        .bind(other)
        .execute(&pool)
        .await;
    assert!(
        second
            .unwrap_err()
            .as_database_error()
            .is_some_and(|error| error.is_unique_violation())
    );
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{donation}/approve"),
            Some(&token(owner, Role::Empresa)),
            None,
        )
        .await
        .0,
        StatusCode::OK
    );
    let approved: (Uuid, String) =
        sqlx::query_as("SELECT ngo_id, status FROM donation_requests WHERE donation_id = $1")
            .bind(donation)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(approved, (assigned, "aprobada".to_string()));
}

#[tokio::test]
async fn don_02_foreign_company_cannot_approve() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let foreign = user(&pool, Role::Empresa).await;
    let ong_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ong_user, true).await;
    let donation = reserved_donation(&pool, owner, ngo_id).await;
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{donation}/approve"),
            Some(&token(foreign, Role::Empresa)),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let status: String = sqlx::query("SELECT status FROM donation_requests WHERE donation_id = $1")
        .bind(donation)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("status");
    assert_eq!(status, "pendiente");
}

#[tokio::test]
async fn don_03_ong_and_ceo_cannot_approve() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let donation = donation(&pool, owner, None).await;
    sqlx::query("UPDATE donations SET status = 'reservado' WHERE id = $1")
        .bind(donation)
        .execute(&pool)
        .await
        .unwrap();
    for role in [Role::Ong, Role::Ceo, Role::Admin] {
        let id = user(&pool, role.clone()).await;
        assert_eq!(
            send(
                &app,
                "POST",
                &format!("/api/donations/{donation}/approve"),
                Some(&token(id, role)),
                None
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
}

#[tokio::test]
async fn don_missing_request_is_controlled() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let donation = donation(&pool, owner, None).await;
    let auth = token(owner, Role::Empresa);
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{donation}/approve"),
            Some(&auth),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{}/approve", Uuid::new_v4()),
            Some(&auth),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn scan_01_assigned_ngo_can_deliver_or_reject() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, true).await;
    for action in ["entrega", "rechazo"] {
        let donation = in_transit_donation(&pool, owner, ngo_id).await;
        assert_eq!(
            send(
                &app,
                "POST",
                "/api/scanner/scan",
                Some(&token(ngo_user, Role::Ong)),
                Some(json!({"donation_id": donation, "action": action, "rejection_reason": "Daño físico"}))
            )
            .await
            .0,
            StatusCode::OK
        );
    }
}

#[tokio::test]
async fn scan_02_foreign_ngo_cannot_modify() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let assigned_user = user(&pool, Role::Ong).await;
    let assigned = ngo(&pool, assigned_user, true).await;
    let foreign = user(&pool, Role::Ong).await;
    let _ = ngo(&pool, foreign, true).await;
    let donation = in_transit_donation(&pool, owner, assigned).await;
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/scanner/scan",
            Some(&token(foreign, Role::Ong)),
            Some(json!({"donation_id": donation, "action": "entrega"}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn scan_03_foreign_company_cannot_modify() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let foreign = user(&pool, Role::Empresa).await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, true).await;
    let donation = approved_reserved_donation(&pool, owner, ngo_id).await;
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/scanner/scan",
            Some(&token(foreign, Role::Empresa)),
            Some(json!({"donation_id": donation, "action": "salida"}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn scan_owner_wrong_role_and_admin_override() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ceo = user(&pool, Role::Ceo).await;
    let ong_user = user(&pool, Role::Ong).await;
    let admin = user(&pool, Role::Admin).await;
    let assigned_user = user(&pool, Role::Ong).await;
    let assigned = ngo(&pool, assigned_user, true).await;
    let donation = approved_reserved_donation(&pool, owner, assigned).await;
    let body = json!({"donation_id": donation, "action": "salida"});
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/scanner/scan",
            Some(&token(owner, Role::Empresa)),
            Some(body.clone())
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/scanner/scan",
            Some(&token(ceo, Role::Ceo)),
            Some(body.clone())
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/scanner/scan",
            Some(&token(admin, Role::Admin)),
            Some(body.clone())
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let admin_donation = approved_reserved_donation(&pool, owner, assigned).await;
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/scanner/scan",
            Some(&token(admin, Role::Admin)),
            Some(json!({"donation_id": admin_donation, "action": "salida"}))
        )
        .await
        .0,
        StatusCode::OK
    );
    let delivery = json!({"donation_id": donation, "action": "entrega"});
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/scanner/scan",
            Some(&token(owner, Role::Empresa)),
            Some(delivery.clone())
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/scanner/scan",
            Some(&token(ong_user, Role::Ong)),
            Some(json!({"donation_id": donation, "action": "salida"}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/scanner/scan",
            Some(&token(admin, Role::Admin)),
            Some(delivery)
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn seed_01_05_admin_only_and_configuration_required() {
    let (app, pool) = app().await;
    assert_eq!(
        send(&app, "POST", "/api/seed/veracruz", None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    for role in [Role::Empresa, Role::Ong, Role::Ceo] {
        let id = user(&pool, role.clone()).await;
        assert_eq!(
            send(
                &app,
                "POST",
                "/api/seed/veracruz",
                Some(&token(id, role)),
                None
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    let admin = user(&pool, Role::Admin).await;
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/seed/veracruz",
            Some(&token(admin, Role::Admin)),
            None
        )
        .await
        .0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn admin_01_04_verify_requires_admin() {
    let (app, pool) = app().await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, false).await;
    for role in [Role::Empresa, Role::Ong, Role::Ceo] {
        let id = if role == Role::Ong {
            ngo_user
        } else {
            user(&pool, role.clone()).await
        };
        assert_eq!(
            send(
                &app,
                "POST",
                &format!("/api/auth/ngos/{ngo_id}/verify"),
                Some(&token(id, role)),
                None
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    let admin = user(&pool, Role::Admin).await;
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/auth/ngos/{ngo_id}/verify"),
            Some(&token(admin, Role::Admin)),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let verified: bool = sqlx::query("SELECT is_verified FROM ngos WHERE id = $1")
        .bind(ngo_id)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("is_verified");
    assert!(verified);
}

#[tokio::test]
async fn admin_list_and_groq_diagnostic_require_admin() {
    let (app, pool) = app().await;
    let ceo = user(&pool, Role::Ceo).await;
    for path in ["/api/auth/ngos", "/api/seed/test-groq"] {
        assert_eq!(
            send(&app, "GET", path, None, None).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send(&app, "GET", path, Some(&token(ceo, Role::Ceo)), None)
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    let admin = user(&pool, Role::Admin).await;
    assert_eq!(
        send(
            &app,
            "GET",
            "/api/auth/ngos",
            Some(&token(admin, Role::Admin)),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
}

fn bootstrap_config(email: &str, password: String) -> ProvisionConfig {
    ProvisionConfig::from_values(Some(email.to_string()), Some(password)).unwrap()
}

#[tokio::test]
async fn admin_boot_01_empty_database_creates_one_admin() {
    let name = format!("ferxarp_admin_test_{}", Uuid::new_v4().simple());
    let bootstrap = PgPoolOptions::new()
        .connect_with(local_options())
        .await
        .unwrap();
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&bootstrap)
        .await
        .unwrap();
    let pool = PgPoolOptions::new()
        .connect_with(local_options().database(&name))
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let initial_count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(initial_count, 0);

    let email = format!("{}@test.local", Uuid::new_v4());
    let password = Uuid::new_v4().to_string();
    let config = bootstrap_config(&email, password.clone());
    assert_eq!(
        provision_admin(&pool, &config).await.unwrap(),
        ProvisionOutcome::Created
    );
    let row = sqlx::query("SELECT role::text AS role, password_hash FROM users WHERE email = $1")
        .bind(&email)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("role"), "admin");
    assert!(bcrypt::verify(password, row.get::<String, _>("password_hash").as_str()).unwrap());
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(total, 1);

    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP DATABASE {name}")))
        .execute(&bootstrap)
        .await
        .unwrap();
    bootstrap.close().await;
}

#[tokio::test]
async fn admin_boot_02_repeated_run_preserves_single_admin_and_hash() {
    let pool = pool().await;
    let email = format!("{}@test.local", Uuid::new_v4());
    let first = bootstrap_config(&email, Uuid::new_v4().to_string());
    let second = bootstrap_config(&email, Uuid::new_v4().to_string());
    assert_eq!(
        provision_admin(&pool, &first).await.unwrap(),
        ProvisionOutcome::Created
    );
    let original_hash: String =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE email = $1")
            .bind(&email)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        provision_admin(&pool, &second).await.unwrap(),
        ProvisionOutcome::AlreadyExists
    );
    let row = sqlx::query(
        "SELECT count(*) AS count, min(password_hash) AS password_hash FROM users WHERE email = $1",
    )
    .bind(&email)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.get::<i64, _>("count"), 1);
    assert_eq!(
        row.get::<Option<String>, _>("password_hash"),
        Some(original_hash)
    );
}

async fn existing_non_admin_is_not_promoted(role: Role) {
    let pool = pool().await;
    let id = user(&pool, role.clone()).await;
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let config = bootstrap_config(&email, Uuid::new_v4().to_string());
    assert_eq!(
        provision_admin(&pool, &config).await,
        Err(ProvisionError::ExistingNonAdmin)
    );
    let actual: String = sqlx::query_scalar("SELECT role::text FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        actual,
        match role {
            Role::Empresa => "empresa",
            Role::Ong => "ong",
            _ => unreachable!(),
        }
    );
}

#[tokio::test]
async fn admin_boot_03_empresa_is_not_promoted() {
    existing_non_admin_is_not_promoted(Role::Empresa).await;
}

#[tokio::test]
async fn admin_boot_04_ong_is_not_promoted() {
    existing_non_admin_is_not_promoted(Role::Ong).await;
}

#[tokio::test]
async fn admin_boot_05_output_and_errors_hide_password() {
    let pool = pool().await;
    let email = format!("{}@test.local", Uuid::new_v4());
    let password = Uuid::new_v4().to_string();
    let config = bootstrap_config(&email, password.clone());
    let outcome = provision_admin(&pool, &config).await.unwrap();
    assert!(!outcome.message().contains(&password));

    let company = user(&pool, Role::Empresa).await;
    let company_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(company)
        .fetch_one(&pool)
        .await
        .unwrap();
    let error = provision_admin(&pool, &bootstrap_config(&company_email, password.clone()))
        .await
        .unwrap_err();
    assert!(!error.to_string().contains(&password));
    assert!(!format!("{error:?}").contains(&password));
}

#[test]
fn admin_boot_06_missing_or_invalid_configuration_is_controlled() {
    assert!(matches!(
        ProvisionConfig::from_values(None, None),
        Err(ProvisionError::MissingEmail)
    ));
    assert!(matches!(
        ProvisionConfig::from_values(Some("admin@test.local".into()), None),
        Err(ProvisionError::MissingPassword)
    ));
    assert!(matches!(
        ProvisionConfig::from_values(Some(" ".into()), Some(Uuid::new_v4().to_string())),
        Err(ProvisionError::MissingEmail)
    ));
    assert!(matches!(
        ProvisionConfig::from_values(Some("invalid".into()), Some(Uuid::new_v4().to_string())),
        Err(ProvisionError::InvalidEmail)
    ));
    assert!(matches!(
        ProvisionConfig::from_values(Some("admin@test.local".into()), Some(String::new())),
        Err(ProvisionError::MissingPassword)
    ));
}

#[tokio::test]
async fn admin_boot_concurrent_runs_create_one_admin() {
    let pool = pool().await;
    let email = format!("{}@test.local", Uuid::new_v4());
    let config = bootstrap_config(&email, Uuid::new_v4().to_string());
    let (first, second) = tokio::join!(
        provision_admin(&pool, &config),
        provision_admin(&pool, &config)
    );
    let outcomes = [first.unwrap(), second.unwrap()];
    assert!(outcomes.contains(&ProvisionOutcome::Created));
    assert!(outcomes.contains(&ProvisionOutcome::AlreadyExists));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn admin_boot_provisioned_admin_can_login_and_list_ngos() {
    let (app, pool) = app().await;
    let email = format!("{}@test.local", Uuid::new_v4());
    let password = Uuid::new_v4().to_string();
    provision_admin(&pool, &bootstrap_config(&email, password.clone()))
        .await
        .unwrap();
    let (status, login) = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"email": email, "password": password})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let jwt = login["token"].as_str().unwrap();
    assert_eq!(
        send(&app, "GET", "/api/auth/ngos", Some(jwt), None).await.0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn admin_boot_enables_initial_donation_chain_without_manual_database_edits() {
    let (app, pool) = app().await;
    let admin_email = format!("{}@test.local", Uuid::new_v4());
    let admin_password = Uuid::new_v4().to_string();
    provision_admin(
        &pool,
        &bootstrap_config(&admin_email, admin_password.clone()),
    )
    .await
    .unwrap();
    let (status, login) = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"email": admin_email, "password": admin_password})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let admin_jwt = login["token"].as_str().unwrap();

    let company_email = format!("{}@test.local", Uuid::new_v4());
    let (status, company) = send(
        &app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({"email": company_email, "password": Uuid::new_v4().to_string(), "role": "empresa"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let company_jwt = company["token"].as_str().unwrap();

    let ngo_email = format!("{}@test.local", Uuid::new_v4());
    let (status, ngo_user) = send(
        &app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({"email": ngo_email, "password": Uuid::new_v4().to_string(), "role": "ong"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let ngo_jwt = ngo_user["token"].as_str().unwrap();
    let ngo_id: Uuid = sqlx::query_scalar(
        "SELECT n.id FROM ngos n JOIN users u ON u.id = n.user_id WHERE u.email = $1",
    )
    .bind(ngo_email)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/auth/ngos/{ngo_id}/verify"),
            Some(admin_jwt),
            None,
        )
        .await
        .0,
        StatusCode::OK
    );

    let (status, donation) = send(
        &app,
        "POST",
        "/api/donations",
        Some(company_jwt),
        Some(json!({"title": "Test donation", "quantity": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let donation_id = donation["id"].as_str().unwrap();
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{donation_id}/request"),
            Some(ngo_jwt),
            None,
        )
        .await
        .0,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn flow_01_02_03_creation_reservation_and_second_request() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let first_user = user(&pool, Role::Ong).await;
    let second_user = user(&pool, Role::Ong).await;
    let first = ngo(&pool, first_user, true).await;
    let _second = ngo(&pool, second_user, true).await;
    let (status, created) = send(
        &app,
        "POST",
        "/api/donations",
        Some(&token(owner, Role::Empresa)),
        Some(json!({"title":"Food", "quantity":1})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["status"], "en_acopio");
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{id}/request"),
            Some(&token(first_user, Role::Ong)),
            None
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert_eq!(
        donation_state(&pool, id).await,
        ("reservado".into(), None, None, false)
    );
    let actual: (Uuid, String) =
        sqlx::query_as("SELECT ngo_id, status FROM donation_requests WHERE donation_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(actual, (first, "pendiente".into()));
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{id}/request"),
            Some(&token(second_user, Role::Ong)),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM donation_requests WHERE donation_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
    assert_eq!(log_count(&pool, id).await, 0);
}

#[tokio::test]
async fn flow_04_05_approval_and_physical_departure_are_distinct() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, true).await;
    let id = reserved_donation(&pool, owner, ngo_id).await;
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{id}/approve"),
            Some(&token(owner, Role::Empresa)),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        donation_state(&pool, id).await,
        ("reservado".into(), Some(ngo_id), None, false)
    );
    let status: String =
        sqlx::query_scalar("SELECT status FROM donation_requests WHERE donation_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "aprobada");
    assert_eq!(log_count(&pool, id).await, 0);
    assert_eq!(
        scan(&app, id, owner, Role::Empresa, "salida", None).await,
        StatusCode::OK
    );
    assert_eq!(donation_state(&pool, id).await.0, "en_transito");
    let log: (String, Option<String>, String) = sqlx::query_as(
        "SELECT action, previous_status, new_status FROM delivery_logs WHERE donation_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        log,
        (
            "salida".into(),
            Some("reservado".into()),
            "en_transito".into()
        )
    );
    assert_eq!(log_count(&pool, id).await, 1);
}

#[tokio::test]
async fn flow_06_departure_without_approved_request_conflicts_without_log() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, true).await;
    let id = reserved_donation(&pool, owner, ngo_id).await;
    assert_eq!(
        scan(&app, id, owner, Role::Empresa, "salida", None).await,
        StatusCode::CONFLICT
    );
    assert_eq!(donation_state(&pool, id).await.0, "reservado");
    assert_eq!(log_count(&pool, id).await, 0);
}

#[tokio::test]
async fn flow_07_delivery_sets_completion_and_one_correct_log() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, true).await;
    let id = in_transit_donation(&pool, owner, ngo_id).await;
    assert_eq!(
        scan(&app, id, ngo_user, Role::Ong, "entrega", None).await,
        StatusCode::OK
    );
    assert_eq!(
        donation_state(&pool, id).await,
        ("entregado".into(), Some(ngo_id), None, true)
    );
    let log: (String, Option<String>, String) = sqlx::query_as(
        "SELECT action, previous_status, new_status FROM delivery_logs WHERE donation_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        log,
        (
            "entrega".into(),
            Some("en_transito".into()),
            "entregado".into()
        )
    );
    assert_eq!(log_count(&pool, id).await, 1);
}

#[tokio::test]
async fn flow_08_09_rejection_requires_nonempty_reason_and_logs_once() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, true).await;
    let id = in_transit_donation(&pool, owner, ngo_id).await;
    for reason in [None, Some("  ")] {
        assert_eq!(
            scan(&app, id, ngo_user, Role::Ong, "rechazo", reason).await,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(donation_state(&pool, id).await.0, "en_transito");
        assert_eq!(log_count(&pool, id).await, 0);
    }
    assert_eq!(
        scan(
            &app,
            id,
            ngo_user,
            Role::Ong,
            "rechazo",
            Some("  Daño físico  ")
        )
        .await,
        StatusCode::OK
    );
    assert_eq!(
        donation_state(&pool, id).await,
        (
            "rechazado".into(),
            Some(ngo_id),
            Some("Daño físico".into()),
            true
        )
    );
    let log: (String, Option<String>, String, Option<String>) = sqlx::query_as("SELECT action, previous_status, new_status, notes FROM delivery_logs WHERE donation_id = $1").bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(
        log,
        (
            "rechazo".into(),
            Some("en_transito".into()),
            "rechazado".into(),
            Some("Daño físico".into())
        )
    );
    assert_eq!(log_count(&pool, id).await, 1);
}

#[tokio::test]
async fn flow_10_15_invalid_and_repeated_transitions_never_log() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, true).await;
    let fresh = donation(&pool, owner, Some(ngo_id)).await;
    assert_eq!(
        scan(&app, fresh, owner, Role::Empresa, "entrada", None).await,
        StatusCode::CONFLICT
    );
    assert_eq!(
        scan(&app, fresh, owner, Role::Empresa, "salida", None).await,
        StatusCode::CONFLICT
    );
    for action in ["entrega", "rechazo"] {
        assert_eq!(
            scan(&app, fresh, ngo_user, Role::Ong, action, Some("reason")).await,
            StatusCode::CONFLICT
        );
    }
    assert_eq!(log_count(&pool, fresh).await, 0);
    let reserved = approved_reserved_donation(&pool, owner, ngo_id).await;
    for action in ["entrega", "rechazo"] {
        assert_eq!(
            scan(&app, reserved, ngo_user, Role::Ong, action, Some("reason")).await,
            StatusCode::CONFLICT
        );
    }
    assert_eq!(
        scan(&app, reserved, owner, Role::Empresa, "entrada", None).await,
        StatusCode::CONFLICT
    );
    assert_eq!(log_count(&pool, reserved).await, 0);
    let delivered = in_transit_donation(&pool, owner, ngo_id).await;
    assert_eq!(
        scan(&app, delivered, ngo_user, Role::Ong, "entrega", None).await,
        StatusCode::OK
    );
    assert_eq!(
        scan(&app, delivered, ngo_user, Role::Ong, "entrega", None).await,
        StatusCode::CONFLICT
    );
    assert_eq!(
        scan(
            &app,
            delivered,
            ngo_user,
            Role::Ong,
            "rechazo",
            Some("reason")
        )
        .await,
        StatusCode::CONFLICT
    );
    assert_eq!(
        scan(&app, delivered, owner, Role::Empresa, "entrada", None).await,
        StatusCode::CONFLICT
    );
    assert_eq!(log_count(&pool, delivered).await, 1);
    let rejected = in_transit_donation(&pool, owner, ngo_id).await;
    assert_eq!(
        scan(
            &app,
            rejected,
            ngo_user,
            Role::Ong,
            "rechazo",
            Some("reason")
        )
        .await,
        StatusCode::OK
    );
    assert_eq!(
        scan(&app, rejected, ngo_user, Role::Ong, "entrega", None).await,
        StatusCode::CONFLICT
    );
    assert_eq!(
        scan(
            &app,
            rejected,
            ngo_user,
            Role::Ong,
            "rechazo",
            Some("reason")
        )
        .await,
        StatusCode::CONFLICT
    );
    assert_eq!(log_count(&pool, rejected).await, 1);
}

#[tokio::test]
async fn conc_01_two_ngos_race_to_reserve_exactly_one_wins() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let first = user(&pool, Role::Ong).await;
    let second = user(&pool, Role::Ong).await;
    let _ = ngo(&pool, first, true).await;
    let _ = ngo(&pool, second, true).await;
    let id = donation(&pool, owner, None).await;
    let uri = format!("/api/donations/{id}/request");
    let first_token = token(first, Role::Ong);
    let second_token = token(second, Role::Ong);
    let (a, b) = tokio::join!(
        send(&app, "POST", &uri, Some(&first_token), None),
        send(&app, "POST", &uri, Some(&second_token), None)
    );
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::CREATED, StatusCode::CONFLICT]);
    assert_eq!(donation_state(&pool, id).await.0, "reservado");
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM donation_requests WHERE donation_id = $1 AND status = 'pendiente'",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn conc_02_two_approvals_change_request_once() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, true).await;
    let id = reserved_donation(&pool, owner, ngo_id).await;
    let uri = format!("/api/donations/{id}/approve");
    let auth = token(owner, Role::Empresa);
    let (a, b) = tokio::join!(
        send(&app, "POST", &uri, Some(&auth), None),
        send(&app, "POST", &uri, Some(&auth), None)
    );
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
    assert_eq!(
        donation_state(&pool, id).await,
        ("reservado".into(), Some(ngo_id), None, false)
    );
    let status: String =
        sqlx::query_scalar("SELECT status FROM donation_requests WHERE donation_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "aprobada");
    assert_eq!(log_count(&pool, id).await, 0);
}

#[tokio::test]
async fn conc_03_two_departures_produce_one_transition_and_log() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, true).await;
    let id = approved_reserved_donation(&pool, owner, ngo_id).await;
    let (a, b) = tokio::join!(
        scan(&app, id, owner, Role::Empresa, "salida", None),
        scan(&app, id, owner, Role::Empresa, "salida", None)
    );
    let mut statuses = [a, b];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
    assert_eq!(donation_state(&pool, id).await.0, "en_transito");
    assert_eq!(log_count(&pool, id).await, 1);
}

#[tokio::test]
async fn conc_04_delivery_and_rejection_race_to_one_terminal_result() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, true).await;
    let id = in_transit_donation(&pool, owner, ngo_id).await;
    let (a, b) = tokio::join!(
        scan(&app, id, ngo_user, Role::Ong, "entrega", None),
        scan(&app, id, ngo_user, Role::Ong, "rechazo", Some("Damaged"))
    );
    let mut statuses = [a, b];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
    let (state, _, reason, completed) = donation_state(&pool, id).await;
    assert!(completed);
    assert!(matches!(state.as_str(), "entregado" | "rechazado"));
    assert_eq!(reason.is_some(), state == "rechazado");
    assert_eq!(log_count(&pool, id).await, 1);
    let logged: String =
        sqlx::query_scalar("SELECT new_status FROM delivery_logs WHERE donation_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(logged, state);
}

#[tokio::test]
async fn rollback_request_approval_and_scan_leave_no_partial_writes_and_logs_are_immutable() {
    let options = local_options();
    let bootstrap = PgPoolOptions::new()
        .connect_with(options.clone())
        .await
        .unwrap();
    let name = format!("ferxarp_lifecycle_{}", Uuid::new_v4().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&bootstrap)
        .await
        .unwrap();
    let isolated = PgPoolOptions::new()
        .max_connections(3)
        .connect_with(options.database(&name))
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&isolated).await.unwrap();
    let app = router_for(&isolated);
    let owner = user(&isolated, Role::Empresa).await;
    let ngo_user = user(&isolated, Role::Ong).await;
    let ngo_id = ngo(&isolated, ngo_user, true).await;

    sqlx::query("CREATE FUNCTION fail_lifecycle_write() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected rollback failure'; END $$")
        .execute(&isolated).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_donation_update BEFORE UPDATE ON donations FOR EACH ROW EXECUTE FUNCTION fail_lifecycle_write()")
        .execute(&isolated).await.unwrap();
    let request_id = donation(&isolated, owner, None).await;
    let (status, response) = send(
        &app,
        "POST",
        &format!("/api/donations/{request_id}/request"),
        Some(&token(ngo_user, Role::Ong)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!response.to_string().contains("injected rollback failure"));
    assert_eq!(donation_state(&isolated, request_id).await.0, "en_acopio");
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM donation_requests WHERE donation_id = $1")
            .bind(request_id)
            .fetch_one(&isolated)
            .await
            .unwrap();
    assert_eq!(count, 0);
    sqlx::query("DROP TRIGGER fail_donation_update ON donations")
        .execute(&isolated)
        .await
        .unwrap();

    let approval_id = reserved_donation(&isolated, owner, ngo_id).await;
    sqlx::query("CREATE TRIGGER fail_donation_update BEFORE UPDATE ON donations FOR EACH ROW EXECUTE FUNCTION fail_lifecycle_write()")
        .execute(&isolated).await.unwrap();
    let (status, response) = send(
        &app,
        "POST",
        &format!("/api/donations/{approval_id}/approve"),
        Some(&token(owner, Role::Empresa)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!response.to_string().contains("injected rollback failure"));
    assert_eq!(
        donation_state(&isolated, approval_id).await,
        ("reservado".into(), None, None, false)
    );
    let request_status: String =
        sqlx::query_scalar("SELECT status FROM donation_requests WHERE donation_id = $1")
            .bind(approval_id)
            .fetch_one(&isolated)
            .await
            .unwrap();
    assert_eq!(request_status, "pendiente");
    sqlx::query("DROP TRIGGER fail_donation_update ON donations")
        .execute(&isolated)
        .await
        .unwrap();

    let scan_id = approved_reserved_donation(&isolated, owner, ngo_id).await;
    sqlx::query("CREATE TRIGGER fail_log_insert BEFORE INSERT ON delivery_logs FOR EACH ROW EXECUTE FUNCTION fail_lifecycle_write()")
        .execute(&isolated).await.unwrap();
    let (status, response) = send(
        &app,
        "POST",
        "/api/scanner/scan",
        Some(&token(owner, Role::Empresa)),
        Some(json!({"donation_id":scan_id,"action":"salida"})),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!response.to_string().contains("injected rollback failure"));
    assert_eq!(donation_state(&isolated, scan_id).await.0, "reservado");
    assert_eq!(log_count(&isolated, scan_id).await, 0);
    sqlx::query("DROP TRIGGER fail_log_insert ON delivery_logs")
        .execute(&isolated)
        .await
        .unwrap();
    assert_eq!(
        scan(&app, scan_id, owner, Role::Empresa, "salida", None).await,
        StatusCode::OK
    );
    let log_id: Uuid = sqlx::query_scalar("SELECT id FROM delivery_logs WHERE donation_id = $1")
        .bind(scan_id)
        .fetch_one(&isolated)
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE delivery_logs SET notes = 'tamper' WHERE id = $1")
            .bind(log_id)
            .execute(&isolated)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM delivery_logs WHERE id = $1")
            .bind(log_id)
            .execute(&isolated)
            .await
            .is_err()
    );
    assert_eq!(log_count(&isolated, scan_id).await, 1);

    isolated.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP DATABASE {name} WITH (FORCE)")))
        .execute(&bootstrap)
        .await
        .unwrap();
    bootstrap.close().await;
}

#[tokio::test]
async fn migration_upgrades_ferxarp_001_rows_and_archives_duplicate_requests() {
    let options = local_options();
    let bootstrap = PgPoolOptions::new()
        .connect_with(options.clone())
        .await
        .unwrap();
    let name = format!("ferxarp_upgrade_{}", Uuid::new_v4().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&bootstrap)
        .await
        .unwrap();
    let isolated = PgPoolOptions::new()
        .connect_with(options.database(&name))
        .await
        .unwrap();
    let all = sqlx::migrate!("./migrations");
    let first = sqlx::migrate::Migrator::with_migrations(vec![all.migrations[0].clone()]);
    first.run(&isolated).await.unwrap();
    let owner = user(&isolated, Role::Empresa).await;
    let ngo_user_a = user(&isolated, Role::Ong).await;
    let ngo_user_b = user(&isolated, Role::Ong).await;
    let ngo_a = ngo(&isolated, ngo_user_a, true).await;
    let ngo_b = ngo(&isolated, ngo_user_b, true).await;
    let id: Uuid = sqlx::query_scalar("INSERT INTO donations (user_id, title, quantity, status, assigned_ngo_id) VALUES ($1, 'legacy', 1, 'en_transito', $2) RETURNING id")
        .bind(owner).bind(ngo_a).fetch_one(&isolated).await.unwrap();
    request(&isolated, id, ngo_a).await;
    request(&isolated, id, ngo_b).await;
    sqlx::query(
        "UPDATE donation_requests SET status = 'aprobada' WHERE donation_id = $1 AND ngo_id = $2",
    )
    .bind(id)
    .bind(ngo_b)
    .execute(&isolated)
    .await
    .unwrap();
    let null_id: Uuid = sqlx::query_scalar("INSERT INTO donations (user_id, title, quantity, status) VALUES ($1, 'null state', 1, NULL) RETURNING id")
        .bind(owner).fetch_one(&isolated).await.unwrap();
    let rejected_id: Uuid = sqlx::query_scalar("INSERT INTO donations (user_id, title, quantity, status) VALUES ($1, 'old rejection', 1, 'rechazado') RETURNING id")
        .bind(owner).fetch_one(&isolated).await.unwrap();
    all.run(&isolated).await.unwrap();
    assert_eq!(
        donation_state(&isolated, id).await,
        ("en_transito".into(), Some(ngo_b), None, false)
    );
    assert_eq!(donation_state(&isolated, null_id).await.0, "en_acopio");
    let (_, _, reason, completed) = donation_state(&isolated, rejected_id).await;
    assert!(completed);
    assert!(reason.unwrap().contains("históricos"));
    let retained: Uuid =
        sqlx::query_scalar("SELECT ngo_id FROM donation_requests WHERE donation_id = $1")
            .bind(id)
            .fetch_one(&isolated)
            .await
            .unwrap();
    assert_eq!(retained, ngo_b);
    let archived: Uuid = sqlx::query_scalar(
        "SELECT ngo_id FROM donation_request_duplicates_archive WHERE donation_id = $1",
    )
    .bind(id)
    .fetch_one(&isolated)
    .await
    .unwrap();
    assert_eq!(archived, ngo_a);
    assert!(
        sqlx::query("INSERT INTO donation_requests (donation_id, ngo_id) VALUES ($1, $2)")
            .bind(id)
            .bind(ngo_a)
            .execute(&isolated)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE donations SET status = NULL WHERE id = $1")
            .bind(null_id)
            .execute(&isolated)
            .await
            .is_err()
    );
    isolated.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP DATABASE {name} WITH (FORCE)")))
        .execute(&bootstrap)
        .await
        .unwrap();
    bootstrap.close().await;
}

#[tokio::test]
async fn persistence_rejects_inconsistent_completion_and_rejection_reason() {
    let pool = pool().await;
    let owner = user(&pool, Role::Empresa).await;
    let id = donation(&pool, owner, None).await;
    assert!(
        sqlx::query("UPDATE donations SET status = 'entregado' WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query(
            "UPDATE donations SET status = 'rechazado', completed_at = now() WHERE id = $1"
        )
        .bind(id)
        .execute(&pool)
        .await
        .is_err()
    );
    assert!(
        sqlx::query("UPDATE donations SET rejection_reason = 'orphan reason' WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .is_err()
    );
    assert_eq!(
        donation_state(&pool, id).await,
        ("en_acopio".into(), None, None, false)
    );
}

#[tokio::test]
async fn full_http_lifecycle_from_publication_to_delivery() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, true).await;
    let (status, created) = send(
        &app,
        "POST",
        "/api/donations",
        Some(&token(owner, Role::Empresa)),
        Some(json!({"title":"Food", "quantity":2})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{id}/request"),
            Some(&token(ngo_user, Role::Ong)),
            None
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/donations/{id}/approve"),
            Some(&token(owner, Role::Empresa)),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        scan(&app, id, owner, Role::Empresa, "salida", None).await,
        StatusCode::OK
    );
    assert_eq!(
        scan(&app, id, ngo_user, Role::Ong, "entrega", None).await,
        StatusCode::OK
    );
    assert_eq!(
        donation_state(&pool, id).await,
        ("entregado".into(), Some(ngo_id), None, true)
    );
    let logs: Vec<(String, Option<String>, String)> = sqlx::query_as("SELECT action, previous_status, new_status FROM delivery_logs WHERE donation_id = $1 ORDER BY created_at, id")
        .bind(id).fetch_all(&pool).await.unwrap();
    assert_eq!(
        logs,
        vec![
            (
                "salida".into(),
                Some("reservado".into()),
                "en_transito".into()
            ),
            (
                "entrega".into(),
                Some("en_transito".into()),
                "entregado".into()
            )
        ]
    );
}

#[tokio::test]
async fn ong_shipments_show_only_own_requests_with_completion_fields() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let ong_user = user(&pool, Role::Ong).await;
    let other_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ong_user, true).await;
    let other_ngo = ngo(&pool, other_user, true).await;
    let own = in_transit_donation(&pool, owner, ngo_id).await;
    let foreign = in_transit_donation(&pool, owner, other_ngo).await;
    assert_eq!(
        scan(&app, own, ong_user, Role::Ong, "entrega", None).await,
        StatusCode::OK
    );
    let (status, shipments) = send(
        &app,
        "GET",
        "/api/donations/shipments",
        Some(&token(ong_user, Role::Ong)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = shipments.as_array().unwrap();
    let own_item = items
        .iter()
        .find(|item| item["donation_id"] == own.to_string())
        .unwrap();
    assert_eq!(own_item["assigned_ngo_id"], ngo_id.to_string());
    assert!(own_item["completed_at"].as_str().is_some());
    assert!(
        !items
            .iter()
            .any(|item| item["donation_id"] == foreign.to_string())
    );
}

#[tokio::test]
async fn donation_lists_preserve_owner_and_public_feed_boundaries() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let other = user(&pool, Role::Empresa).await;
    let available = donation(&pool, owner, None).await;
    let reserved = donation(&pool, owner, None).await;
    let foreign = donation(&pool, other, None).await;
    sqlx::query("UPDATE donations SET status = 'reservado' WHERE id = $1")
        .bind(reserved)
        .execute(&pool)
        .await
        .unwrap();

    let (status, own) = send(
        &app,
        "GET",
        "/api/donations",
        Some(&token(owner, Role::Empresa)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let own = own.as_array().unwrap();
    assert!(own.iter().any(|item| item["id"] == available.to_string()));
    assert!(own.iter().any(|item| item["id"] == reserved.to_string()));
    assert!(!own.iter().any(|item| item["id"] == foreign.to_string()));

    let (status, feed) = send(
        &app,
        "GET",
        "/api/donations/feed",
        Some(&token(owner, Role::Empresa)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let feed = feed.as_array().unwrap();
    assert!(feed.iter().any(|item| item["id"] == available.to_string()));
    assert!(feed.iter().any(|item| item["id"] == foreign.to_string()));
    assert!(!feed.iter().any(|item| item["id"] == reserved.to_string()));
    assert!(feed.iter().all(|item| item["status"] == "en_acopio"));
}

#[tokio::test]
async fn shipment_views_limit_companies_and_allow_admin_ceo_audit() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let other = user(&pool, Role::Empresa).await;
    let ngo_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ngo_user, true).await;
    let own = reserved_donation(&pool, owner, ngo_id).await;
    let foreign = reserved_donation(&pool, other, ngo_id).await;

    let (status, shipments) = send(
        &app,
        "GET",
        "/api/donations/shipments",
        Some(&token(owner, Role::Empresa)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = shipments.as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|item| item["donation_id"] == own.to_string())
    );
    assert!(
        !items
            .iter()
            .any(|item| item["donation_id"] == foreign.to_string())
    );

    for role in [Role::Admin, Role::Ceo] {
        let auditor = user(&pool, role.clone()).await;
        let (status, shipments) = send(
            &app,
            "GET",
            "/api/donations/shipments",
            Some(&token(auditor, role)),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let items = shipments.as_array().unwrap();
        assert!(
            items
                .iter()
                .any(|item| item["donation_id"] == own.to_string())
        );
        assert!(
            items
                .iter()
                .any(|item| item["donation_id"] == foreign.to_string())
        );
    }
}
