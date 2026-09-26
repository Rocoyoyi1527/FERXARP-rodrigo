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
    models::{
        donation_state::{DonationState, PhysicalAction},
        user::Role,
    },
};

#[derive(Deserialize)]
pub struct ScanRequest {
    pub donation_id: Uuid,
    pub action: ScanAction,
    pub rejection_reason: Option<String>,
    pub notes: Option<String>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanAction {
    Entrada, // Ingreso al centro de acopio
    Salida,  // En tránsito hacia la ONG
    Entrega, // Recepción final exitosa
    Rechazo, // Merma o rechazo por condiciones físicas
}

impl ScanAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScanAction::Entrada => "entrada",
            ScanAction::Salida => "salida",
            ScanAction::Entrega => "entrega",
            ScanAction::Rechazo => "rechazo",
        }
    }
}

impl From<ScanAction> for PhysicalAction {
    fn from(action: ScanAction) -> Self {
        match action {
            ScanAction::Entrada => Self::Entrada,
            ScanAction::Salida => Self::Salida,
            ScanAction::Entrega => Self::Entrega,
            ScanAction::Rechazo => Self::Rechazo,
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

    let previous = DonationState::from_db(&current.status).ok_or_else(internal_error)?;
    let next = previous
        .after_physical_action(payload.action.into())
        .ok_or((StatusCode::CONFLICT, "Transición no permitida".to_string()))?;

    let reason = if matches!(payload.action, ScanAction::Rechazo) {
        let value = payload
            .rejection_reason
            .as_deref()
            .unwrap_or_default()
            .trim();
        if value.is_empty() || value.chars().count() > 500 {
            return Err((
                StatusCode::BAD_REQUEST,
                "Motivo de rechazo inválido".to_string(),
            ));
        }
        Some(value.to_string())
    } else {
        None
    };

    if matches!(payload.action, ScanAction::Salida) {
        let ngo_id = current
            .assigned_ngo_id
            .ok_or((StatusCode::CONFLICT, "Salida sin ONG aprobada".to_string()))?;
        let approved = sqlx::query!(
            "SELECT id FROM donation_requests WHERE donation_id = $1 AND ngo_id = $2 AND status = 'aprobada'",
            payload.donation_id,
            ngo_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
        if approved.is_none() {
            return Err((
                StatusCode::CONFLICT,
                "Salida sin solicitud aprobada".to_string(),
            ));
        }
    }

    if matches!(payload.action, ScanAction::Entrega | ScanAction::Rechazo)
        && current.assigned_ngo_id.is_none()
    {
        return Err((StatusCode::CONFLICT, "Entrega sin ONG asignada".to_string()));
    }

    let new_status = next.as_str();
    let updated = sqlx::query!(
        r#"
        UPDATE donations 
        SET status = $1,
            rejection_reason = $4,
            completed_at = CASE WHEN $1 IN ('entregado', 'rechazado') THEN now() ELSE NULL END
        WHERE id = $2 AND status = $3
        "#,
        new_status,
        payload.donation_id,
        previous.as_str(),
        reason.as_deref()
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    if updated.rows_affected() != 1 {
        return Err((StatusCode::CONFLICT, "Transición no permitida".to_string()));
    }

    sqlx::query!(
        r#"
        INSERT INTO delivery_logs (donation_id, action, previous_status, new_status, notes)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        payload.donation_id,
        payload.action.as_str(),
        previous.as_str(),
        new_status,
        if matches!(payload.action, ScanAction::Rechazo) {
            reason
        } else {
            payload.notes
        }
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;

    Ok(Json(ScanResponse {
        donation_id: payload.donation_id,
        previous_status: Some(current.status),
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
