use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
};
use bcrypt::{DEFAULT_COST, hash};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    AppState,
    ai::{
        chroma_db::ChromaClient,
        groq::GroqClient,
        index::{NgoIndexRecord, upsert_ngo},
    },
    api::{
        auth::Claims,
        middleware::{internal_error, require_role},
    },
    models::user::Role,
};

#[derive(Serialize)]
pub struct AiMatchEvaluation {
    pub donation_title: String,
    pub recommended_ngo: String,
    pub score: f64,
    pub priority: String,
    pub reasoning: String,
}

#[derive(Serialize)]
pub struct SeedResponse {
    pub message: String,
    pub companies_seeded: usize,
    pub ngos_seeded: usize,
    pub donations_seeded: usize,
    pub ai_evaluations: Vec<AiMatchEvaluation>,
}

#[derive(Default, Deserialize)]
struct SeedOptions {
    include_ai: Option<bool>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/veracruz", post(seed_veracruz_data))
        .route("/test-groq", get(test_groq_connection))
}

mod data;

// POST /api/seed/veracruz — fixtures ficticios; nunca descubre organizaciones.
async fn seed_veracruz_data(
    claims: Claims,
    Query(options): Query<SeedOptions>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<SeedResponse>, (StatusCode, String)> {
    require_role(&claims, Role::Admin)?;
    let seed_password = state.seed_password.as_deref().ok_or_else(internal_error)?;
    let password_hash = hash(seed_password, DEFAULT_COST).map_err(|_| internal_error())?;
    let chroma = state
        .chroma_url
        .as_deref()
        .and_then(|url| ChromaClient::new(url).ok())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "ChromaDB no está configurada".into(),
        ))?;
    chroma.health().await.map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "ChromaDB no está disponible".into(),
        )
    })?;

    // A transaction-scoped lock also serializes simultaneous Admin invocations.
    let mut tx = state.db.begin().await.map_err(|_| internal_error())?;
    sqlx::query("SELECT pg_advisory_xact_lock(70620260929)")
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    let mut created = 0;
    for (i, email) in data::COMPANIES.iter().enumerate() {
        ensure_demo_user(&mut tx, demo_id(1, i), email, "empresa", &password_hash).await?;
    }
    for (i, ngo) in data::NGOS.iter().enumerate() {
        let user_id = demo_id(2, i);
        ensure_demo_user(&mut tx, user_id, ngo.email, "ong", &password_hash).await?;
        sqlx::query("INSERT INTO ngos (id, user_id, name, needs_description, latitude, longitude, is_verified) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (id) DO NOTHING")
            .bind(demo_id(3, i)).bind(user_id).bind(ngo.name)
            .bind(format!("DEMO FICTICIA · coordenadas aproximadas y verificación simulada, sin vínculo con organizaciones reales. {}", ngo.needs))
            .bind(ngo.lat).bind(ngo.lon).bind(i < 9)
            .execute(&mut *tx).await.map_err(|_| internal_error())?;
        let owner: Uuid = sqlx::query_scalar("SELECT user_id FROM ngos WHERE id=$1")
            .bind(demo_id(3, i))
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
        if owner != user_id {
            return Err(seed_collision());
        }
    }
    for (i, donation) in data::DONATIONS.iter().enumerate() {
        created += insert_demo_donation(&mut tx, i, donation).await?;
    }
    tx.commit().await.map_err(|_| internal_error())?;

    // PostgreSQL has committed; index errors are recoverable using reindex_chroma.
    let mut ngos_list = Vec::new();
    for (i, ngo) in data::NGOS.iter().enumerate() {
        let record = sqlx::query_as::<_, NgoIndexRecord>(
            "SELECT id, name, needs_description FROM ngos WHERE id=$1",
        )
        .bind(demo_id(3, i))
        .fetch_one(&state.db)
        .await
        .map_err(|_| internal_error())?;
        upsert_ngo(&chroma, &record).await.map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Datos demo guardados; ejecute reindex_chroma para completar el índice".into(),
            )
        })?;
        ngos_list.push((
            record.id,
            record.name,
            record.needs_description.unwrap_or_default(),
            ngo.lat,
            ngo.lon,
        ));
    }
    let all_created_donations: Vec<_> = data::DONATIONS
        .iter()
        .enumerate()
        .map(|(i, d)| {
            (
                demo_id(4, i),
                d.title.to_string(),
                d.description.to_string(),
            )
        })
        .collect();
    // Enriquecimiento opcional; no se invoca durante la demo por defecto.
    let groq = GroqClient::new();
    let mut ai_evaluations = Vec::new();

    for (_don_id, title, desc) in
        all_created_donations
            .iter()
            .take(if options.include_ai.unwrap_or(false) {
                4
            } else {
                0
            })
    {
        let mut best_score = 0.0;
        let mut best_eval: Option<AiMatchEvaluation> = None;

        for (_ngo_id, name, needs, lat, lon) in &ngos_list {
            let dist = ((19.1738 - lat).powi(2) + (-96.1342 - lon).powi(2)).sqrt() * 111.0;

            if let Some(eval) = groq.evaluate_fit(title, desc, name, needs, dist).await
                && eval.compatibility_score > best_score
            {
                best_score = eval.compatibility_score;
                best_eval = Some(AiMatchEvaluation {
                    donation_title: title.clone(),
                    recommended_ngo: name.clone(),
                    score: eval.compatibility_score,
                    priority: eval.priority_level,
                    reasoning: eval.reasoning,
                });
            }
        }

        if let Some(e) = best_eval {
            ai_evaluations.push(e);
        }
    }

    Ok(Json(SeedResponse {
        message: "Dataset ficticio demo preparado e indexado. Los registros existentes se conservan; Groq es opcional.".to_string(),
        companies_seeded: data::COMPANIES.len(),
        ngos_seeded: data::NGOS.len(),
        donations_seeded: created,
        ai_evaluations,
    }))
}

// GET /api/seed/test-groq - Diagnóstico directo de DeepSeek-R1
async fn test_groq_connection(
    claims: Claims,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_role(&claims, Role::Admin)?;
    let groq = GroqClient::new();
    let start = std::time::Instant::now();

    let eval = groq
        .evaluate_fit(
            "80 Cajas de Leche Entera",
            "Lácteos pasteurizados con 20 días de vigencia",
            "Banco Comunitario Demo Veracruz",
            "Demanda crítica de leche, lácteos y fórmulas infantiles para comedores comunitarios",
            2.4,
        )
        .await;

    let elapsed_ms = start.elapsed().as_millis();

    match eval {
        Some(result) => Ok(Json(serde_json::json!({
            "status": "connected",
            "model": "deepseek-r1-distill-llama-70b",
            "latency_ms": elapsed_ms,
            "evaluation": result
        }))),
        None => Err(internal_error()),
    }
}

// Stable fixture namespace. It never adopts existing accounts by email or name.
fn demo_id(kind: u128, index: usize) -> Uuid {
    Uuid::from_u128(0xfea00000_0000_4000_8000_000000000000 | (kind << 32) | index as u128)
}

fn seed_collision() -> (StatusCode, String) {
    (
        StatusCode::CONFLICT,
        "Identidad demo en conflicto; no se modificaron datos existentes".into(),
    )
}

async fn ensure_demo_user(
    db: &mut sqlx::PgConnection,
    id: Uuid,
    email: &str,
    role: &str,
    password: &str,
) -> Result<(), (StatusCode, String)> {
    sqlx::query("INSERT INTO users(id,email,password_hash,role) VALUES ($1,$2,$3,$4::text::user_role) ON CONFLICT (id) DO NOTHING")
        .bind(id).bind(email).bind(password).bind(role)
        .execute(&mut *db).await.map_err(|_| seed_collision())?;
    let actual: (String, String) = sqlx::query_as("SELECT email,role::text FROM users WHERE id=$1")
        .bind(id)
        .fetch_one(&mut *db)
        .await
        .map_err(|_| internal_error())?;
    if actual != (email.to_owned(), role.to_owned()) {
        return Err(seed_collision());
    }
    Ok(())
}

async fn insert_demo_donation(
    db: &mut sqlx::PgConnection,
    index: usize,
    donation: &data::DonationSeed,
) -> Result<usize, (StatusCode, String)> {
    use crate::models::donation_state::{DonationState, PhysicalAction};
    let id = demo_id(4, index);
    let company = demo_id(1, donation.company);
    let ngo = demo_id(3, donation.ngo);
    let existing: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM donations WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut *db)
        .await
        .map_err(|_| internal_error())?;
    if let Some(owner) = existing {
        if owner != company {
            return Err(seed_collision());
        }
        // Preserve rehearsed lifecycle, names, verification and passwords on reruns.
        return Ok(0);
    }
    if donation.status != "en_acopio" {
        let verified: bool = sqlx::query_scalar("SELECT is_verified FROM ngos WHERE id=$1")
            .bind(ngo)
            .fetch_one(&mut *db)
            .await
            .map_err(|_| internal_error())?;
        if !verified {
            return Err((
                StatusCode::CONFLICT,
                "El fixture requiere una ONG demo verificada; no se revierte una revocación".into(),
            ));
        }
    }
    sqlx::query("INSERT INTO donations(id,user_id,title,description,quantity,status) VALUES ($1,$2,$3,$4,$5,'en_acopio')")
        .bind(id).bind(company).bind(donation.title)
        .bind(format!("DEMO FICTICIA · no acredita operaciones ni impacto real. {}", donation.description)).bind(donation.quantity)
        .execute(&mut *db).await.map_err(|_| internal_error())?;
    if donation.status == "en_acopio" {
        return Ok(1);
    }
    sqlx::query("INSERT INTO donation_requests(id,donation_id,ngo_id,status) VALUES ($1,$2,$3,$4)")
        .bind(demo_id(5, index))
        .bind(id)
        .bind(ngo)
        .bind(if donation.approved {
            "aprobada"
        } else {
            "pendiente"
        })
        .execute(&mut *db)
        .await
        .map_err(|_| internal_error())?;
    sqlx::query("UPDATE donations SET status='reservado', assigned_ngo_id=$2 WHERE id=$1")
        .bind(id)
        .bind(donation.approved.then_some(ngo))
        .execute(&mut *db)
        .await
        .map_err(|_| internal_error())?;
    let mut status = DonationState::Reservado;
    let actions: &[PhysicalAction] = match donation.status {
        "reservado" => &[],
        "en_transito" => &[PhysicalAction::Salida],
        "entregado" => &[PhysicalAction::Salida, PhysicalAction::Entrega],
        "rechazado" => &[PhysicalAction::Salida, PhysicalAction::Rechazo],
        _ => return Err(internal_error()),
    };
    if !actions.is_empty() && !donation.approved {
        return Err(internal_error());
    }
    for (step, action) in actions.iter().enumerate() {
        let next = status
            .after_physical_action(*action)
            .ok_or_else(internal_error)?;
        let action_name = match action {
            PhysicalAction::Salida => "salida",
            PhysicalAction::Entrega => "entrega",
            PhysicalAction::Rechazo => "rechazo",
            _ => return Err(internal_error()),
        };
        let note = if next == DonationState::Rechazado {
            "DEMO FICTICIA · rechazo simulado por embalaje dañado o piezas faltantes."
        } else {
            "DEMO FICTICIA · transición simulada para exposición; no es una entrega real."
        };
        sqlx::query("UPDATE donations SET status=$2, completed_at=CASE WHEN $2 IN ('entregado','rechazado') THEN now() ELSE NULL END, rejection_reason=$3 WHERE id=$1")
            .bind(id).bind(next.as_str()).bind((next == DonationState::Rechazado).then_some(note))
            .execute(&mut *db).await.map_err(|_| internal_error())?;
        sqlx::query("INSERT INTO delivery_logs(id,donation_id,action,previous_status,new_status,notes) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(demo_id(6, index * 2 + step)).bind(id).bind(action_name).bind(status.as_str()).bind(next.as_str()).bind(note)
            .execute(&mut *db).await.map_err(|_| internal_error())?;
        status = next;
    }
    Ok(1)
}
