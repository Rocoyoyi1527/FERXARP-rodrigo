use axum::{
    Router,
    http::{HeaderValue, Method, header},
    routing::get,
};
use dotenv::dotenv;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use std::env;
use std::sync::Arc;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    set_header::SetResponseHeaderLayer,
};
use tracing::info;

pub use backend::ai;
pub mod api;
pub mod models;

// Estado compartido inyectado en los controladores de Axum
pub struct AppState {
    pub db: Pool<Postgres>,
    pub jwt_secret: String,
    pub seed_password: Option<String>,
    pub chroma_url: Option<String>,
}

#[tokio::main]
async fn main() {
    // 1. Inicializar sistema de logs y variables de entorno
    tracing_subscriber::fmt::init();
    dotenv().ok();

    info!("Iniciando el servidor principal de Fexarp...");

    // 2. Extraer credenciales desde .env
    let database_url = env::var("DATABASE_URL").expect("Falta DATABASE_URL en el archivo .env");
    let jwt_secret = env::var("JWT_SECRET").expect("Falta JWT_SECRET en el archivo .env");
    let seed_password = env::var("FERXARP_SEED_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty());
    let chroma_url = env::var("CHROMA_URL")
        .ok()
        .filter(|value| !value.trim().is_empty());

    // 3. Establecer conexión con Supabase (PostgreSQL)
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Error crítico: No se pudo conectar a Supabase");

    info!("Conexión a PostgreSQL (Supabase) establecida exitosamente.");

    // 4. Empaquetar estado compartido
    let shared_state = Arc::new(AppState {
        db: pool,
        jwt_secret,
        seed_password,
        chroma_url,
    });

    let app = build_router(shared_state);

    // 6. Levantar el servidor en el puerto 8000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    info!("Servidor escuchando en http://localhost:8000");
    axum::serve(listener, app).await.unwrap();
}

fn cors_layer(frontend_origin: &str) -> CorsLayer {
    let origin = HeaderValue::from_str(frontend_origin)
        .expect("FRONTEND_ORIGIN debe ser un origen HTTP válido sin caracteres de control");
    assert!(
        (frontend_origin.starts_with("http://") || frontend_origin.starts_with("https://"))
            && !frontend_origin.ends_with('/')
            && !frontend_origin.contains('*'),
        "FRONTEND_ORIGIN debe ser un origen HTTP(S) concreto, sin ruta ni comodines"
    );
    CorsLayer::new()
        .allow_origin(AllowOrigin::list([origin]))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .expose_headers([header::HeaderName::from_static("x-matching-mode")])
}

fn build_router(shared_state: Arc<AppState>) -> Router {
    let frontend_origin =
        env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".to_owned());
    Router::new()
        .route("/health", get(health_check))
        .nest("/api/auth", api::auth::router())
        .nest("/api/donations", api::donations::router())
        .nest("/api/scanner", api::scanner::router())
        .nest("/api/metrics", api::metrics::router())
        .nest("/api/seed", api::seed::router())
        .layer(cors_layer(&frontend_origin))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .with_state(shared_state)
}

// Endpoint de verificación rápida del servidor
async fn health_check() -> &'static str {
    "¡Fexarp API Online! El cerebro en Rust está conectado a Supabase."
}

#[cfg(test)]
mod security_tests;
