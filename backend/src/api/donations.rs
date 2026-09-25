use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    AppState,
    ai::{
        chroma_db::ChromaClient,
        groq::GroqClient,
        matcher::{NgoCandidate, rank_ngos_for_donation},
    },
    api::{
        auth::Claims,
        middleware::{internal_error, require_role},
    },
    models::{donation_state::DonationState, user::Role},
};

// --- ESTRUCTURAS DE DATOS ---

#[derive(Deserialize)]
pub struct CreateDonationRequest {
    pub title: String,
    pub description: Option<String>,
    pub quantity: i32,
}

#[derive(Serialize, Deserialize)]
pub struct DonationResponse {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub quantity: i32,
    pub status: String,
    pub assigned_ngo_id: Option<Uuid>,
}

#[derive(Serialize, Deserialize)]
pub struct FeedDonationResponse {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub quantity: i32,
    pub status: String,
    pub donor_email: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize)]
pub struct ShipmentItem {
    pub id: Uuid,
    pub donation_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub quantity: i32,
    pub donation_status: String,
    pub request_status: String,
    pub donor_email: String,
    pub ngo_name: String,
    pub rejection_reason: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScoredMatchWithAi {
    pub ngo_id: Uuid,
    pub ngo_name: String,
    pub distance_km: f64,
    pub semantic_similarity: f64,
    pub final_score: f64,
    pub ai_reasoning: Option<String>,
    pub ai_priority: Option<String>,
}

// --- ENRUTADOR ---

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_donation).get(list_donations))
        .route("/feed", get(list_available_feed))
        .route("/shipments", get(list_shipments))
        .route("/{id}/matches", get(get_donation_matches))
        .route("/{id}/request", post(request_donation))
        .route("/{id}/approve", post(approve_shipment))
}

// --- CONTROLADORES CRUD ---

async fn create_donation(
    claims: Claims,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateDonationRequest>,
) -> Result<(StatusCode, Json<DonationResponse>), (StatusCode, String)> {
    if !matches!(claims.role, Role::Empresa | Role::Admin) {
        return Err((
            StatusCode::FORBIDDEN,
            "Acceso denegado: únicamente empresas o administradores pueden publicar donaciones."
                .to_string(),
        ));
    }

    let record = sqlx::query!(
        r#"
        INSERT INTO donations (user_id, title, description, quantity, status) 
        VALUES ($1, $2, $3, $4, 'en_acopio') 
        RETURNING id, user_id, title, description, quantity, status, assigned_ngo_id
        "#,
        claims.sub,
        payload.title,
        payload.description,
        payload.quantity
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| internal_error())?;

    Ok((
        StatusCode::CREATED,
        Json(DonationResponse {
            id: record.id,
            user_id: Some(record.user_id),
            title: record.title,
            description: record.description,
            quantity: record.quantity,
            status: record.status,
            assigned_ngo_id: record.assigned_ngo_id,
        }),
    ))
}

async fn list_donations(
    claims: Claims,
    State(state): State<Arc<AppState>>,
) -> Result<(StatusCode, Json<Vec<DonationResponse>>), (StatusCode, String)> {
    let records = sqlx::query!(
        r#"
        SELECT id, user_id, title, description, quantity, status, assigned_ngo_id 
        FROM donations 
        WHERE user_id = $1 
        ORDER BY created_at DESC
        "#,
        claims.sub
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| internal_error())?;

    let donations = records
        .into_iter()
        .map(|rec| DonationResponse {
            id: rec.id,
            user_id: Some(rec.user_id),
            title: rec.title,
            description: rec.description,
            quantity: rec.quantity,
            status: rec.status,
            assigned_ngo_id: rec.assigned_ngo_id,
        })
        .collect();

    Ok((StatusCode::OK, Json(donations)))
}

async fn list_available_feed(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<FeedDonationResponse>>, (StatusCode, String)> {
    let records = sqlx::query!(
        r#"
        SELECT 
            d.id, 
            d.title, 
            d.description, 
            d.quantity, 
            d.status,
            d.created_at, 
            u.email as donor_email
        FROM donations d
        JOIN users u ON d.user_id = u.id
        WHERE d.status = 'en_acopio'
        ORDER BY d.created_at DESC
        LIMIT 50
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| internal_error())?;

    let feed = records
        .into_iter()
        .map(|r| FeedDonationResponse {
            id: r.id,
            title: r.title,
            description: r.description,
            quantity: r.quantity,
            status: r.status,
            donor_email: r.donor_email,
            created_at: Some(r.created_at),
        })
        .collect();

    Ok(Json(feed))
}

// GET /api/donations/{id}/matches - Matching híbrido (Léxico + ChromaDB + DeepSeek-R1)
async fn get_donation_matches(
    _claims: Claims,
    Path(donation_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ScoredMatchWithAi>>, (StatusCode, String)> {
    let donation = sqlx::query!(
        r#"SELECT title, description FROM donations WHERE id = $1"#,
        donation_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Donación no encontrada".to_string()))?;

    let search_text = format!(
        "{} {}",
        donation.title,
        donation.description.clone().unwrap_or_default()
    )
    .to_lowercase();

    // 1. Similitud vectorial en ChromaDB (Vec<(Uuid, f64)>)
    let chroma = ChromaClient::new(None);
    let mut similarities = chroma
        .query_similar_ngos(&search_text, 10)
        .await
        .unwrap_or_default();

    // 2. Candidatos en Supabase
    let ngos_records =
        sqlx::query!(r#"SELECT id, name, needs_description, latitude, longitude FROM ngos"#)
            .fetch_all(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Boost léxico directo sobre Vec<(Uuid, f64)>
    for ngo in &ngos_records {
        let needs_lower = ngo
            .needs_description
            .clone()
            .unwrap_or_default()
            .to_lowercase();
        let has_direct_match = (search_text.contains("leche") && needs_lower.contains("leche"))
            || (search_text.contains("alimento") && needs_lower.contains("alimento"))
            || (search_text.contains("abarrote") && needs_lower.contains("alimento"))
            || (search_text.contains("fruta") && needs_lower.contains("alimento"))
            || (search_text.contains("servidor") && needs_lower.contains("computadora"))
            || (search_text.contains("laptop") && needs_lower.contains("computadora"));

        if has_direct_match {
            if let Some(pos) = similarities.iter().position(|(id, _)| *id == ngo.id) {
                similarities[pos].1 = 0.94;
            } else {
                similarities.push((ngo.id, 0.94));
            }
        }
    }

    let candidates: Vec<NgoCandidate> = ngos_records
        .into_iter()
        .map(|r| NgoCandidate {
            id: r.id,
            name: r.name,
            needs_description: r.needs_description,
            latitude: r.latitude,
            longitude: r.longitude,
            urgency_level: 4,
        })
        .collect();

    let ranked = rank_ngos_for_donation(19.1738, -96.1342, &similarities, &candidates, 50.0);

    // 3. Puntuación y razonamiento con DeepSeek-R1 (Groq)
    let groq = GroqClient::new();
    let mut enriched_matches = Vec::new();

    for (idx, m) in ranked.into_iter().enumerate() {
        let mut final_score = m.final_score;
        let mut ai_reasoning = None;
        let mut ai_priority = None;

        if idx < 4 {
            let candidate_needs = candidates
                .iter()
                .find(|c| c.id == m.ngo_id)
                .and_then(|c| c.needs_description.as_deref())
                .unwrap_or("");

            if let Some(eval) = groq
                .evaluate_fit(
                    &donation.title,
                    donation.description.as_deref().unwrap_or(""),
                    &m.ngo_name,
                    candidate_needs,
                    m.distance_km,
                )
                .await
            {
                final_score = (eval.compatibility_score * 0.6) + (m.final_score * 0.4);
                ai_reasoning = Some(eval.reasoning);
                ai_priority = Some(eval.priority_level);
            }
        }

        enriched_matches.push(ScoredMatchWithAi {
            ngo_id: m.ngo_id,
            ngo_name: m.ngo_name,
            distance_km: m.distance_km,
            semantic_similarity: m.semantic_similarity,
            final_score,
            ai_reasoning,
            ai_priority,
        });
    }

    enriched_matches.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap());
    Ok(Json(enriched_matches))
}

// POST /api/donations/{id}/request - Solicitud de donación
async fn request_donation(
    claims: Claims,
    Path(donation_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_role(&claims, Role::Ong)?;
    let mut tx = state.db.begin().await.map_err(|_| internal_error())?;
    // Lock the donation first, matching approval's lock order.
    let donation = sqlx::query!(
        "SELECT status, assigned_ngo_id FROM donations WHERE id = $1 FOR UPDATE",
        donation_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| internal_error())?
    .ok_or((StatusCode::NOT_FOUND, "Donación no disponible".to_string()))?;
    let ngo = sqlx::query!(
        "SELECT id, is_verified FROM ngos WHERE user_id = $1 FOR UPDATE",
        claims.sub
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| internal_error())?
    .ok_or((StatusCode::FORBIDDEN, "Perfil ONG requerido".to_string()))?;
    if !ngo.is_verified {
        return Err((StatusCode::FORBIDDEN, "ONG no verificada".to_string()));
    }
    let ngo_id = ngo.id;
    let status = DonationState::from_db(&donation.status).ok_or_else(internal_error)?;
    if !status.can_reserve() || donation.assigned_ngo_id.is_some() {
        return Err((
            StatusCode::CONFLICT,
            "Donación no disponible para reserva".to_string(),
        ));
    }

    let existing = sqlx::query!(
        "SELECT id FROM donation_requests WHERE donation_id = $1",
        donation_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    if existing.is_some() {
        return Err((StatusCode::CONFLICT, "Donación ya solicitada".to_string()));
    }

    sqlx::query!(
        r#"
        INSERT INTO donation_requests (donation_id, ngo_id, status)
        VALUES ($1, $2, 'pendiente')
        "#,
        donation_id,
        ngo_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|db| db.is_unique_violation())
        {
            (StatusCode::CONFLICT, "Donación ya solicitada".to_string())
        } else {
            internal_error()
        }
    })?;

    let updated = sqlx::query!(
        r#"
        UPDATE donations 
        SET status = 'reservado'
        WHERE id = $1 AND status = 'en_acopio' AND assigned_ngo_id IS NULL
        "#,
        donation_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    if updated.rows_affected() != 1 {
        return Err((
            StatusCode::CONFLICT,
            "Donación no disponible para reserva".to_string(),
        ));
    }

    tx.commit().await.map_err(|_| internal_error())?;

    Ok(StatusCode::CREATED)
}

// GET /api/donations/shipments - Listado para la pantalla de tracking
async fn list_shipments(
    claims: Claims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ShipmentItem>>, (StatusCode, String)> {
    let shipments: Vec<ShipmentItem> = match claims.role {
        Role::Empresa => {
            let records = sqlx::query!(
                r#"
                SELECT 
                    r.id as request_id,
                    d.id as donation_id,
                    d.title,
                    d.description,
                    d.quantity,
                    d.status as donation_status,
                    r.status as request_status,
                    u.email as donor_email,
                    n.name as ngo_name,
                    d.rejection_reason,
                    r.created_at
                FROM donation_requests r
                JOIN donations d ON r.donation_id = d.id
                JOIN users u ON d.user_id = u.id
                JOIN ngos n ON r.ngo_id = n.id
                WHERE d.user_id = $1
                ORDER BY r.created_at DESC
                "#,
                claims.sub
            )
            .fetch_all(&state.db)
            .await
            .map_err(|_| internal_error())?;

            records
                .into_iter()
                .map(|r| ShipmentItem {
                    id: r.request_id,
                    donation_id: r.donation_id,
                    title: r.title,
                    description: r.description,
                    quantity: r.quantity,
                    donation_status: r.donation_status,
                    request_status: r.request_status,
                    donor_email: r.donor_email,
                    ngo_name: r.ngo_name,
                    rejection_reason: r.rejection_reason,
                    created_at: Some(r.created_at),
                })
                .collect()
        }
        Role::Ong => {
            let records = sqlx::query!(
                r#"
                SELECT 
                    r.id as request_id,
                    d.id as donation_id,
                    d.title,
                    d.description,
                    d.quantity,
                    d.status as donation_status,
                    r.status as request_status,
                    u.email as donor_email,
                    n.name as ngo_name,
                    d.rejection_reason,
                    r.created_at
                FROM donation_requests r
                JOIN donations d ON r.donation_id = d.id
                JOIN users u ON d.user_id = u.id
                JOIN ngos n ON r.ngo_id = n.id
                WHERE n.user_id = $1
                ORDER BY r.created_at DESC
                "#,
                claims.sub
            )
            .fetch_all(&state.db)
            .await
            .map_err(|_| internal_error())?;

            records
                .into_iter()
                .map(|r| ShipmentItem {
                    id: r.request_id,
                    donation_id: r.donation_id,
                    title: r.title,
                    description: r.description,
                    quantity: r.quantity,
                    donation_status: r.donation_status,
                    request_status: r.request_status,
                    donor_email: r.donor_email,
                    ngo_name: r.ngo_name,
                    rejection_reason: r.rejection_reason,
                    created_at: Some(r.created_at),
                })
                .collect()
        }
        Role::Admin | Role::Ceo => {
            let records = sqlx::query!(
                r#"
                SELECT 
                    r.id as request_id,
                    d.id as donation_id,
                    d.title,
                    d.description,
                    d.quantity,
                    d.status as donation_status,
                    r.status as request_status,
                    u.email as donor_email,
                    n.name as ngo_name,
                    d.rejection_reason,
                    r.created_at
                FROM donation_requests r
                JOIN donations d ON r.donation_id = d.id
                JOIN users u ON d.user_id = u.id
                JOIN ngos n ON r.ngo_id = n.id
                ORDER BY r.created_at DESC
                "#
            )
            .fetch_all(&state.db)
            .await
            .map_err(|_| internal_error())?;

            records
                .into_iter()
                .map(|r| ShipmentItem {
                    id: r.request_id,
                    donation_id: r.donation_id,
                    title: r.title,
                    description: r.description,
                    quantity: r.quantity,
                    donation_status: r.donation_status,
                    request_status: r.request_status,
                    donor_email: r.donor_email,
                    ngo_name: r.ngo_name,
                    rejection_reason: r.rejection_reason,
                    created_at: Some(r.created_at),
                })
                .collect()
        }
    };

    Ok(Json(shipments))
}

// POST /api/donations/{id}/approve - Aprobación del donante, sin salida física
async fn approve_shipment(
    claims: Claims,
    Path(donation_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_role(&claims, Role::Empresa)?;
    let mut tx = state.db.begin().await.map_err(|_| internal_error())?;
    let donation = sqlx::query!(
        "SELECT status, assigned_ngo_id FROM donations WHERE id = $1 AND user_id = $2 FOR UPDATE",
        donation_id,
        claims.sub
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| internal_error())?
    .ok_or((StatusCode::NOT_FOUND, "Donación no disponible".to_string()))?;
    let status = DonationState::from_db(&donation.status).ok_or_else(internal_error)?;
    if !status.can_approve() {
        return Err((
            StatusCode::CONFLICT,
            "Donación fuera de estado reservado".to_string(),
        ));
    }

    let request = sqlx::query!(
        r#"
        SELECT r.id, r.ngo_id, r.status, n.is_verified
        FROM donation_requests r
        JOIN ngos n ON n.id = r.ngo_id
        WHERE r.donation_id = $1
        FOR UPDATE OF r, n
        "#,
        donation_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| internal_error())?
    .ok_or((StatusCode::NOT_FOUND, "Solicitud no disponible".to_string()))?;
    if request.status != "pendiente" || !request.is_verified || donation.assigned_ngo_id.is_some() {
        return Err((StatusCode::CONFLICT, "Solicitud no aprobable".to_string()));
    }

    let updated = sqlx::query!(
        r#"
        UPDATE donation_requests
        SET status = 'aprobada'
        WHERE id = $1 AND status = 'pendiente'
        "#,
        request.id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    if updated.rows_affected() != 1 {
        return Err((StatusCode::CONFLICT, "Solicitud no aprobable".to_string()));
    }

    let updated = sqlx::query!(
        r#"
        UPDATE donations
        SET assigned_ngo_id = $1
        WHERE id = $2 AND status = 'reservado' AND assigned_ngo_id IS NULL
        "#,
        request.ngo_id,
        donation_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    if updated.rows_affected() != 1 {
        return Err((StatusCode::CONFLICT, "Donación no aprobable".to_string()));
    }

    tx.commit().await.map_err(|_| internal_error())?;

    Ok(StatusCode::OK)
}
