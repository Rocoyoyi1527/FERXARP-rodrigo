use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;
use std::sync::Arc;

use crate::{AppState, api::auth::Claims};

#[derive(Serialize)]
pub struct ImpactMetrics {
    pub total_donations: i64,
    pub delivered_donations: i64,
    pub in_transit_donations: i64,
    pub rejected_donations: i64,
    pub total_volume_kg: i64,
    pub estimated_co2_saved_kg: f64,
    pub estimated_beneficiaries: i64,
    pub verified_ngos: i64,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/summary", get(get_impact_summary))
}

// GET /api/metrics/summary - Retorna KPIs consolidados de impacto ambiental y social
async fn get_impact_summary(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ImpactMetrics>, (StatusCode, String)> {
    // 1. Conteo agregado de donaciones por estado
    let stats = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE status = 'entregado') as delivered,
            COUNT(*) FILTER (WHERE status = 'en_transito') as in_transit,
            COUNT(*) FILTER (WHERE status = 'rechazado') as rejected,
            COALESCE(SUM(quantity) FILTER (WHERE status = 'entregado'), 0) as delivered_qty
        FROM donations
        "#
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error BD: {}", e),
        )
    })?;

    // 2. Conteo de ONGs verificadas activas
    let ngos = sqlx::query!(r#"SELECT COUNT(*) as verified FROM ngos WHERE is_verified = true"#)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error BD: {}", e),
            )
        })?;

    let delivered_volume = stats.delivered_qty.unwrap_or(0);
    // Factor de mitigación ambiental: ~2.5 kg de CO2eq evitados por cada kg de residuo recuperado
    let co2_saved = (delivered_volume as f64) * 2.5;
    // Factor de impacto humano: promedio estimado de 5 kg de alimento/material por beneficiario
    let beneficiaries = delivered_volume / 5;

    Ok(Json(ImpactMetrics {
        total_donations: stats.total.unwrap_or(0),
        delivered_donations: stats.delivered.unwrap_or(0),
        in_transit_donations: stats.in_transit.unwrap_or(0),
        rejected_donations: stats.rejected.unwrap_or(0),
        total_volume_kg: delivered_volume,
        estimated_co2_saved_kg: co2_saved,
        estimated_beneficiaries: beneficiaries,
        verified_ngos: ngos.verified.unwrap_or(0),
    }))
}
