// backend/src/ai/matcher.rs

use super::scoring::{calculate_haversine_distance, calculate_match_score};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NgoCandidate {
    pub id: Uuid,
    pub name: String,
    pub needs_description: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub urgency_level: u8, // 1 a 5
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScoredMatch {
    pub ngo_id: Uuid,
    pub ngo_name: String,
    pub final_score: f64,
    pub distance_km: f64,
    pub semantic_similarity: f64,
}

/// Evalúa un conjunto de ONGs contra una donación y devuelve el ranking ordenado por prioridad descendente.
pub fn rank_ngos_for_donation(
    donation_lat: f64,
    donation_lon: f64,
    semantic_similarities: &[(Uuid, f64)], // Vector con (ngo_id, similitud_0_a_1)
    candidates: &[NgoCandidate],
    max_radius_km: f64,
) -> Vec<ScoredMatch> {
    let mut results: Vec<ScoredMatch> = candidates
        .iter()
        .filter_map(|ngo| {
            let lat = ngo.latitude?;
            let lon = ngo.longitude?;

            let distance = calculate_haversine_distance(donation_lat, donation_lon, lat, lon);

            // Buscar la similitud calculada o asumir 0.0 si no hubo coincidencia
            let similarity = semantic_similarities
                .iter()
                .find(|(id, _)| *id == ngo.id)
                .map(|(_, s)| *s)
                .unwrap_or(0.0);

            let score =
                calculate_match_score(similarity, distance, max_radius_km, ngo.urgency_level);

            Some(ScoredMatch {
                ngo_id: ngo.id,
                ngo_name: ngo.name.clone(),
                final_score: (score * 100.0).round() / 100.0,
                distance_km: (distance * 100.0).round() / 100.0,
                semantic_similarity: similarity,
            })
        })
        .collect();

    // Ordenar de mayor a menor puntuación
    results.sort_by(|a, b| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}
