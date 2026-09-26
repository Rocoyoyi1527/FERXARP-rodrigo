// backend/src/ai/matcher.rs

use super::{
    embedding::terms,
    index::{NgoIndexRecord, build_ngo_semantic_document},
    scoring::{calculate_haversine_distance, calculate_match_score},
};
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
    pub distance_km: Option<f64>,
    pub semantic_similarity: f64,
    pub lexical_score: f64,
    pub vector_score: f64,
}

pub const LEXICAL_WEIGHT: f64 = 0.4;
pub const VECTOR_WEIGHT: f64 = 0.6;

pub fn lexical_score(query: &str, needs: &str) -> f64 {
    let query_terms = terms(query);
    let needs_terms = terms(needs);
    if query_terms.is_empty() || needs_terms.is_empty() {
        return 0.0;
    }
    let intersection = query_terms.intersection(&needs_terms).count();
    let union = query_terms.union(&needs_terms).count();
    intersection as f64 / union as f64
}

pub fn combined_similarity(lexical: f64, vector: Option<f64>) -> f64 {
    match vector {
        Some(vector) => (LEXICAL_WEIGHT * lexical + VECTOR_WEIGHT * vector).clamp(0.0, 1.0),
        None => lexical.clamp(0.0, 1.0),
    }
}

/// Evalúa un conjunto de ONGs contra una donación y devuelve el ranking ordenado por prioridad descendente.
pub fn rank_ngos_for_donation(
    donation_lat: f64,
    donation_lon: f64,
    query_text: &str,
    vector_similarities: Option<&[(Uuid, f64)]>,
    candidates: &[NgoCandidate],
    max_radius_km: f64,
) -> Vec<ScoredMatch> {
    let mut results: Vec<ScoredMatch> = candidates
        .iter()
        .filter_map(|ngo| {
            let distance = ngo.latitude.zip(ngo.longitude).map(|(lat, lon)| {
                calculate_haversine_distance(donation_lat, donation_lon, lat, lon)
            });

            let document = build_ngo_semantic_document(&NgoIndexRecord {
                id: ngo.id,
                name: ngo.name.clone(),
                needs_description: ngo.needs_description.clone(),
            });
            let lexical = lexical_score(query_text, &document);
            let vector = vector_similarities.and_then(|matches| {
                matches
                    .iter()
                    .find(|(id, _)| *id == ngo.id)
                    .map(|(_, score)| *score)
            });
            if lexical == 0.0 && vector.unwrap_or(0.0) == 0.0 {
                return None;
            }
            let similarity =
                combined_similarity(lexical, vector_similarities.map(|_| vector.unwrap_or(0.0)));

            let score = calculate_match_score(
                similarity,
                distance.unwrap_or(max_radius_km),
                max_radius_km,
                ngo.urgency_level,
            );

            Some(ScoredMatch {
                ngo_id: ngo.id,
                ngo_name: ngo.name.clone(),
                final_score: (score * 100.0).round() / 100.0,
                distance_km: distance.map(|value| (value * 100.0).round() / 100.0),
                semantic_similarity: similarity,
                lexical_score: lexical,
                vector_score: vector.unwrap_or(0.0),
            })
        })
        .collect();

    // Ordenar de mayor a menor puntuación
    results.sort_by(|a, b| {
        b.final_score
            .total_cmp(&a.final_score)
            .then_with(|| a.ngo_id.cmp(&b.ngo_id))
    });
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_scoring_recognizes_domain_synonyms() {
        assert!(lexical_score("Cajas de leche", "Necesitamos lácteos") > 0.0);
        assert_eq!(lexical_score("laptops", "cobijas"), 0.0);
    }

    #[test]
    fn combined_scoring_has_explicit_weights() {
        assert!((combined_similarity(1.0, Some(0.5)) - 0.7).abs() < 0.000001);
        assert_eq!(combined_similarity(0.8, None), 0.8);
    }

    #[test]
    fn highest_score_ranks_first() {
        let milk = NgoCandidate {
            id: Uuid::new_v4(),
            name: "Leche".into(),
            needs_description: Some("leche".into()),
            latitude: Some(19.0),
            longitude: Some(-96.0),
            urgency_level: 4,
        };
        let computers = NgoCandidate {
            id: Uuid::new_v4(),
            name: "Cómputo".into(),
            needs_description: Some("computadoras".into()),
            latitude: Some(19.0),
            longitude: Some(-96.0),
            urgency_level: 4,
        };
        let ranked =
            rank_ngos_for_donation(19.0, -96.0, "leche", None, &[computers, milk.clone()], 50.0);
        assert_eq!(ranked[0].ngo_id, milk.id);
    }

    #[test]
    fn ngo_without_coordinates_still_receives_content_score() {
        let ngo = NgoCandidate {
            id: Uuid::new_v4(),
            name: "Banco".into(),
            needs_description: Some("leche".into()),
            latitude: None,
            longitude: None,
            urgency_level: 4,
        };
        let ranked = rank_ngos_for_donation(19.0, -96.0, "leche", None, &[ngo], 50.0);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].distance_km, None);
        assert!(ranked[0].final_score > 0.0);
    }

    #[test]
    fn unrelated_candidate_is_not_reported_as_a_match() {
        let ngo = NgoCandidate {
            id: Uuid::new_v4(),
            name: "Centro Escolar".into(),
            needs_description: Some("computadoras".into()),
            latitude: Some(19.0),
            longitude: Some(-96.0),
            urgency_level: 5,
        };
        let ranked = rank_ngos_for_donation(19.0, -96.0, "leche", None, &[ngo], 50.0);
        assert!(ranked.is_empty());
    }
}
