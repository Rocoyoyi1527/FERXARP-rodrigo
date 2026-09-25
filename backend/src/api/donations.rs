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
    api::auth::Claims,
    models::user::Role,
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
    pub status: Option<String>,
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
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error en BD: {}", e),
        )
    })?;

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
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error en BD: {}", e),
        )
    })?;

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
            COALESCE(d.status, 'en_acopio') as "status!", 
            d.created_at, 
            u.email as donor_email
        FROM donations d
        JOIN users u ON d.user_id = u.id
        WHERE d.status = 'en_acopio' OR d.status IS NULL
        ORDER BY d.created_at DESC
        LIMIT 50
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
    if !matches!(claims.role, Role::Ong | Role::Admin) {
        return Err((
            StatusCode::FORBIDDEN,
            "Únicamente organizaciones sociales o administradores pueden solicitar donaciones."
                .to_string(),
        ));
    }

    // Auto-creación de registro en la tabla ngos si el usuario no lo tenía
    let ngo_id = match sqlx::query!(r#"SELECT id FROM ngos WHERE user_id = $1"#, claims.sub)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error BD: {}", e),
            )
        })? {
        Some(record) => record.id,
        None => {
            let user_email = sqlx::query!(r#"SELECT email FROM users WHERE id = $1"#, claims.sub)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Error BD: {}", e),
                    )
                })?
                .map(|u| u.email)
                .unwrap_or_else(|| "Organización Social".to_string());

            let name = user_email.split('@').next().unwrap_or("Organización");
            let new_id = Uuid::new_v4();

            sqlx::query!(
                r#"
                INSERT INTO ngos (id, user_id, name, needs_description, latitude, longitude, is_verified)
                VALUES ($1, $2, $3, $4, 19.1738, -96.1342, true)
                ON CONFLICT (id) DO NOTHING
                "#,
                new_id,
                claims.sub,
                name,
                "Recepción comunitaria y distribución de donativos"
            )
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error al inicializar ONG: {}", e)))?;

            new_id
        }
    };

    sqlx::query!(
        r#"
        INSERT INTO donation_requests (donation_id, ngo_id, status)
        VALUES ($1, $2, 'pendiente')
        ON CONFLICT (donation_id, ngo_id) DO NOTHING
        "#,
        donation_id,
        ngo_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error al crear solicitud: {}", e),
        )
    })?;

    sqlx::query!(
        r#"
        UPDATE donations 
        SET assigned_ngo_id = $1, status = 'reservado' 
        WHERE id = $2 AND (status = 'en_acopio' OR status IS NULL)
        "#,
        ngo_id,
        donation_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error al apartar donación: {}", e),
        )
    })?;

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
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error BD: {}", e),
                )
            })?;

            records
                .into_iter()
                .map(|r| ShipmentItem {
                    id: r.request_id,
                    donation_id: r.donation_id,
                    title: r.title,
                    description: r.description,
                    quantity: r.quantity,
                    donation_status: r.donation_status.unwrap_or_else(|| "reservado".to_string()),
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
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error BD: {}", e),
                )
            })?;

            records
                .into_iter()
                .map(|r| ShipmentItem {
                    id: r.request_id,
                    donation_id: r.donation_id,
                    title: r.title,
                    description: r.description,
                    quantity: r.quantity,
                    donation_status: r.donation_status.unwrap_or_else(|| "reservado".to_string()),
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
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error BD: {}", e),
                )
            })?;

            records
                .into_iter()
                .map(|r| ShipmentItem {
                    id: r.request_id,
                    donation_id: r.donation_id,
                    title: r.title,
                    description: r.description,
                    quantity: r.quantity,
                    donation_status: r.donation_status.unwrap_or_else(|| "reservado".to_string()),
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

// POST /api/donations/{id}/approve - Aprobación de salida
async fn approve_shipment(
    claims: Claims,
    Path(donation_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, String)> {
    if !matches!(claims.role, Role::Empresa | Role::Admin) {
        return Err((
            StatusCode::FORBIDDEN,
            "Únicamente la empresa donante o un administrador pueden autorizar el despacho."
                .to_string(),
        ));
    }

    sqlx::query!(
        r#"
        UPDATE donation_requests 
        SET status = 'aprobada' 
        WHERE donation_id = $1
        "#,
        donation_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error al aprobar solicitud: {}", e),
        )
    })?;

    sqlx::query!(
        r#"
        UPDATE donations 
        SET status = 'en_transito' 
        WHERE id = $1
        "#,
        donation_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error al actualizar envío: {}", e),
        )
    })?;

    sqlx::query!(
        r#"
        INSERT INTO delivery_logs (donation_id, action, previous_status, new_status, notes)
        VALUES ($1, 'salida', 'reservado', 'en_transito', 'Despacho autorizado por la empresa donante')
        "#,
        donation_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error en bitácora: {}", e)))?;

    Ok(StatusCode::OK)
}
