use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use bcrypt::{DEFAULT_COST, hash, verify};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{AppState, models::user::Role};

// --- ESTRUCTURAS DE DATOS ---

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub role: Role,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub role: Role,
    pub exp: usize,
}

// --- ENRUTADOR ---

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
        .route("/ngos", get(list_all_ngos))
        .route("/ngos/{id}/verify", post(toggle_ngo_verification))
}

// --- CONTROLADORES ---

// POST /api/auth/register - Registro de usuario, auto-alta de ONG y expedición de JWT[cite: 8, 14]
async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    // 1. Encriptar contraseña
    let hashed_password = hash(&payload.password, DEFAULT_COST)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 2. Insertar en Supabase manejando el conflicto de email duplicado
    let user_record = sqlx::query!(
        r#"
        INSERT INTO users (email, password_hash, role) 
        VALUES ($1, $2, $3)
        RETURNING id, role as "role: Role"
        "#,
        payload.email,
        hashed_password,
        payload.role as Role
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error()
            && db_err.is_unique_violation()
        {
            return (
                StatusCode::CONFLICT,
                "Este correo electrónico ya se encuentra registrado. Por favor inicia sesión."
                    .to_string(),
            );
        }
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error al crear usuario: {}", e),
        )
    })?;

    // 3. Si el rol registrado es ONG, inicializar automáticamente su registro relacional[cite: 14, 16]
    if user_record.role == Role::Ong {
        let default_name = payload.email.split('@').next().unwrap_or("Organización");
        let _ = sqlx::query!(
            r#"
            INSERT INTO ngos (user_id, name, needs_description, latitude, longitude, is_verified)
            VALUES ($1, $2, $3, $4, $5, false)
            ON CONFLICT DO NOTHING
            "#,
            user_record.id,
            default_name,
            "Recepción y distribución comunitaria de excedentes e insumos",
            19.1738,
            -96.1342
        )
        .execute(&state.db)
        .await;
    }

    // 4. Generar token JWT con vigencia de 24 horas[cite: 8]
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("Timestamp inválido")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_record.id,
        role: user_record.role,
        exp: expiration,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_ref()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "token": token,
            "message": "Usuario registrado exitosamente"
        })),
    ))
}

// POST /api/auth/login - Autenticación y retorno de JWT[cite: 8]
async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, String)> {
    let user_record = sqlx::query!(
        r#"SELECT id, password_hash, role as "role: Role" FROM users WHERE email = $1"#,
        payload.email
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((
        StatusCode::UNAUTHORIZED,
        "Credenciales incorrectas".to_string(),
    ))?;

    let is_valid = verify(&payload.password, &user_record.password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !is_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Credenciales incorrectas".to_string(),
        ));
    }

    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("Timestamp inválido")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_record.id,
        role: user_record.role,
        exp: expiration,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_ref()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::OK, Json(AuthResponse { token })))
}

// GET /api/auth/me - Consulta del token y rol actual[cite: 8]
async fn me(claims: Claims) -> Result<Json<Claims>, (StatusCode, String)> {
    Ok(Json(claims))
}

// GET /api/auth/ngos - Auditoría de ONGs para el Administrador de TI (HU-4)[cite: 13]
async fn list_all_ngos(
    claims: Claims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, String)> {
    if claims.role != Role::Admin {
        return Err((
            StatusCode::FORBIDDEN,
            "Acceso restringido: se requieren privilegios de Administrador de TI.".to_string(),
        ));
    }

    let records = sqlx::query!(
        r#"
        SELECT n.id, n.name, n.needs_description, n.is_verified, n.created_at, u.email
        FROM ngos n
        JOIN users u ON n.user_id = u.id
        ORDER BY n.created_at DESC
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error BD: {}", e),
        )
    })?;

    let list = records
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "needs_description": r.needs_description,
                "is_verified": r.is_verified,
                "email": r.email,
                "created_at": r.created_at
            })
        })
        .collect();

    Ok(Json(list))
}

// POST /api/auth/ngos/{id}/verify - Alternar verificación oficial de una ONG[cite: 13, 16]
async fn toggle_ngo_verification(
    claims: Claims,
    Path(ngo_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if claims.role != Role::Admin {
        return Err((
            StatusCode::FORBIDDEN,
            "Acceso restringido: se requieren privilegios de Administrador de TI.".to_string(),
        ));
    }

    let updated = sqlx::query!(
        r#"
        UPDATE ngos 
        SET is_verified = NOT COALESCE(is_verified, false) 
        WHERE id = $1 
        RETURNING id, is_verified
        "#,
        ngo_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error al verificar ONG: {}", e),
        )
    })?;

    Ok(Json(serde_json::json!({
        "id": updated.id,
        "is_verified": updated.is_verified
    })))
}
