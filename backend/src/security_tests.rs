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

use crate::{AppState, api::auth::Claims, models::user::Role};
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
    (
        super::build_router(Arc::new(AppState {
            db: pool.clone(),
            jwt_secret: SECRET.to_string(),
            seed_password: None,
        })),
        pool,
    )
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
    let donation = donation(&pool, owner, Some(ngo_id)).await;
    request(&pool, donation, ngo_id).await;
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
}

#[tokio::test]
async fn don_approval_updates_only_assigned_ngo_request() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let assigned_user = user(&pool, Role::Ong).await;
    let other_user = user(&pool, Role::Ong).await;
    let assigned = ngo(&pool, assigned_user, true).await;
    let other = ngo(&pool, other_user, true).await;
    let donation = donation(&pool, owner, Some(assigned)).await;
    request(&pool, donation, assigned).await;
    request(&pool, donation, other).await;
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
    let other_status: String =
        sqlx::query("SELECT status FROM donation_requests WHERE donation_id = $1 AND ngo_id = $2")
            .bind(donation)
            .bind(other)
            .fetch_one(&pool)
            .await
            .unwrap()
            .get("status");
    assert_eq!(other_status, "pendiente");
}

#[tokio::test]
async fn don_02_foreign_company_cannot_approve() {
    let (app, pool) = app().await;
    let owner = user(&pool, Role::Empresa).await;
    let foreign = user(&pool, Role::Empresa).await;
    let ong_user = user(&pool, Role::Ong).await;
    let ngo_id = ngo(&pool, ong_user, true).await;
    let donation = donation(&pool, owner, Some(ngo_id)).await;
    request(&pool, donation, ngo_id).await;
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
        StatusCode::NOT_FOUND
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
        let donation = donation(&pool, owner, Some(ngo_id)).await;
        assert_eq!(
            send(
                &app,
                "POST",
                "/api/scanner/scan",
                Some(&token(ngo_user, Role::Ong)),
                Some(json!({"donation_id": donation, "action": action}))
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
    let donation = donation(&pool, owner, Some(assigned)).await;
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
    let donation = donation(&pool, owner, None).await;
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
    let donation = donation(&pool, owner, None).await;
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
            Some(body)
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
