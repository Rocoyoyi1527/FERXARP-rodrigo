use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    AppState,
    api::{auth::Claims, middleware::internal_error},
    models::user::Role,
};

#[derive(Deserialize)]
pub struct ScanRequest {
    pub donation_id: Uuid,
    pub action: ScanAction,
    pub rejection_reason: Option<String>,
    pub notes: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanAction {
    Entrada, // Ingreso al centro de acopio
    Salida,  // En tránsito hacia la ONG
    Entrega, // Recepción final exitosa
    Rechazo, // Merma o rechazo por condiciones físicas
}

impl ScanAction {
    pub fn to_status(&self) -> &'static str {
        match self {
            ScanAction::Entrada => "en_acopio",
            ScanAction::Salida => "en_transito",
            ScanAction::Entrega => "entregado",
            ScanAction::Rechazo => "rechazado",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ScanAction::Entrada => "entrada",
            ScanAction::Salida => "salida",
            ScanAction::Entrega => "entrega",
            ScanAction::Rechazo => "rechazo",
        }
    }
}

#[derive(Serialize)]
pub struct ScanResponse {
    pub donation_id: Uuid,
    pub previous_status: Option<String>,
    pub new_status: String,
    pub message: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/scan", post(process_scan))
        .route("/tracking/{id}", get(get_tracking_status))
        .route("/map-points", get(get_map_locations))
}

// POST /api/scanner/scan - Procesa lectura física y audita en delivery_logs
async fn process_scan(
    claims: Claims,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ScanRequest>,
) -> Result<Json<ScanResponse>, (StatusCode, String)> {
    // Entrada/Salida: Empresa propietaria o Admin. Entrega/Rechazo: ONG asignada o Admin.
    let allowed_role = matches!(
        (&payload.action, &claims.role),
        (_, Role::Admin)
            | (ScanAction::Entrada | ScanAction::Salida, Role::Empresa)
            | (ScanAction::Entrega | ScanAction::Rechazo, Role::Ong)
    );
    if !allowed_role {
        return Err((StatusCode::FORBIDDEN, "Acceso denegado".to_string()));
    }

    let new_status = payload.action.to_status();
    let mut tx = state.db.begin().await.map_err(|_| internal_error())?;

    // 1. Obtener estado previo
    let current = sqlx::query!(
        r#"SELECT status, user_id, assigned_ngo_id FROM donations WHERE id = $1 FOR UPDATE"#,
        payload.donation_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| internal_error())?
    .ok_or((
        StatusCode::NOT_FOUND,
        "Lote de donación no encontrado".to_string(),
    ))?;

    match claims.role {
        Role::Empresa if current.user_id != claims.sub => {
            return Err((StatusCode::NOT_FOUND, "Lote no disponible".to_string()));
        }
        Role::Ong => {
            let ngo_id = current
                .assigned_ngo_id
                .ok_or((StatusCode::NOT_FOUND, "Lote no disponible".to_string()))?;
            let assigned = sqlx::query!(
                "SELECT id FROM ngos WHERE id = $1 AND user_id = $2",
                ngo_id,
                claims.sub
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
            if assigned.is_none() {
                return Err((StatusCode::NOT_FOUND, "Lote no disponible".to_string()));
            }
        }
        _ => {}
    }

    // 2. Actualizar donación y registrar fecha de entrega o motivo de rechazo
    sqlx::query!(
        r#"
        UPDATE donations 
        SET status = $1,
            rejection_reason = CASE WHEN $1 = 'rechazado' THEN $3 ELSE rejection_reason END,
            completed_at = CASE WHEN $1 = 'entregado' THEN now() ELSE completed_at END
        WHERE id = $2
        "#,
        new_status,
        payload.donation_id,
        payload.rejection_reason
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    // 3. Registrar auditoría inmutable en delivery_logs (Riesgo R5)
    sqlx::query!(
        r#"
        INSERT INTO delivery_logs (donation_id, action, previous_status, new_status, notes)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        payload.donation_id,
        payload.action.as_str(),
        current.status,
        new_status,
        payload.notes.or(payload.rejection_reason)
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;

    Ok(Json(ScanResponse {
        donation_id: payload.donation_id,
        previous_status: current.status,
        new_status: new_status.to_string(),
        message: format!("Lote actualizado a: {}", new_status),
    }))
}

// GET /api/scanner/tracking/{id}
async fn get_tracking_status(
    _claims: Claims,
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let record = sqlx::query!(
        r#"SELECT id, title, quantity, status, rejection_reason FROM donations WHERE id = $1"#,
        id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error BD: {}", e),
        )
    })?
    .ok_or((StatusCode::NOT_FOUND, "Lote no encontrado".to_string()))?;

    Ok(Json(serde_json::json!({
        "id": record.id,
        "title": record.title,
        "quantity": record.quantity,
        "status": record.status,
        "rejection_reason": record.rejection_reason
    })))
}

// GET /api/scanner/map-points
async fn get_map_locations(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, String)> {
    let mut points = Vec::new();

    points.push(serde_json::json!({
        "id": Uuid::nil(),
        "name": "Centro de Acopio Central Fexarp",
        "point_type": "acopio",
        "latitude": 19.1738,
        "longitude": -96.1342,
        "details": "Hub logístico de recepción y despacho"
    }));

    let ngos = sqlx::query!(
        r#"SELECT id, name, latitude, longitude, needs_description FROM ngos WHERE latitude IS NOT NULL AND longitude IS NOT NULL"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error BD: {}", e)))?;

    for ngo in ngos {
        points.push(serde_json::json!({
            "id": ngo.id,
            "name": ngo.name,
            "point_type": "ong",
            "latitude": ngo.latitude.unwrap_or(19.1738),
            "longitude": ngo.longitude.unwrap_or(-96.1342),
            "details": ngo.needs_description.unwrap_or_else(|| "Recepción de donaciones".to_string())
        }));
    }

    Ok(Json(points))
}
