use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header::AUTHORIZATION, request::Parts},
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use std::sync::Arc;

use crate::models::user::Role;
use crate::{AppState, api::auth::Claims};

pub type ApiError = (StatusCode, String);

pub fn internal_error() -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Error interno".to_string(),
    )
}

pub fn require_role(claims: &Claims, role: Role) -> Result<(), ApiError> {
    if claims.role == role {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, "Acceso denegado".to_string()))
    }
}

// Al usar Axum 0.7+ y Rust moderno, ya no necesitamos la macro #[async_trait]
impl FromRequestParts<Arc<AppState>> for Claims {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // 1. Extraer el header Authorization
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .filter(|value| value.starts_with("Bearer "))
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "Falta el token de autorización o el formato es inválido".to_string(),
            ))?;

        // 2. Limpiar la cadena para obtener solo el token
        let token = auth_header.trim_start_matches("Bearer ");

        // 3. Decodificar y validar criptográficamente el JWT
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Token inválido o expirado".to_string(),
            )
        })?;

        // 4. Inyectar los Claims (con el rol y UUID) directamente al controlador
        Ok(token_data.claims)
    }
}
