use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use bcrypt::{DEFAULT_COST, hash};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    AppState,
    ai::{chroma_db::ChromaClient, groq::GroqClient},
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

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/veracruz", post(seed_veracruz_data))
        .route("/test-groq", get(test_groq_connection))
}

#[allow(dead_code)]
struct CompanySeed {
    email: &'static str,
    name: &'static str,
    lat: f64,
    lon: f64,
    donations: Vec<(&'static str, &'static str, i32)>,
}

struct NgoSeed {
    email: &'static str,
    name: &'static str,
    needs: &'static str,
    lat: f64,
    lon: f64,
}

// POST /api/seed/veracruz - Ingesta idempotente con evaluación DeepSeek-R1
async fn seed_veracruz_data(
    claims: Claims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<SeedResponse>, (StatusCode, String)> {
    require_role(&claims, Role::Admin)?;
    let seed_password = state.seed_password.as_deref().ok_or_else(internal_error)?;
    let default_password = hash(seed_password, DEFAULT_COST).map_err(|_| internal_error())?;

    let companies = vec![
        CompanySeed {
            email: "donaciones@chedraui-americas.com",
            name: "Chedraui Las Américas (Boca del Río)",
            lat: 19.1417,
            lon: -96.1042,
            donations: vec![
                (
                    "80 Cajas de Leche Entera",
                    "Lácteos pasteurizados sellados con 20 días de vigencia para consumo",
                    80,
                ),
                (
                    "120 Paquetes de Abarrotes y Harinas",
                    "Bolsas de harina de trigo, arroz y frijol negro empaquetados",
                    120,
                ),
                (
                    "50 Cajas de Manzanas y Verduras",
                    "Fruta de temporada fresca en cajas de madera aptas para consumo inmediato",
                    50,
                ),
            ],
        },
        CompanySeed {
            email: "sustentabilidad@tenaristamsa.com",
            name: "TenarisTamsa (Parque Industrial Tejería)",
            lat: 19.1764,
            lon: -96.2238,
            donations: vec![
                (
                    "15 Laptops y Servidores Dell",
                    "Equipo de cómputo funcional retirado por ciclo de renovación interna",
                    15,
                ),
                (
                    "40 Escritorios y Sillas de Oficina",
                    "Mobiliario ergonómico en óptimo estado para aulas o administración",
                    40,
                ),
            ],
        },
        CompanySeed {
            email: "contacto@cafiver.com",
            name: "Cafiver Veracruz",
            lat: 19.1650,
            lon: -96.1400,
            donations: vec![(
                "200 Frascos de Café Soluble",
                "Café procesado en frascos herméticos de 200g listos para despensas",
                200,
            )],
        },
    ];

    let ngos = vec![
        NgoSeed {
            email: "direccion@bancodealimentosveracruz.org",
            name: "Banco de Alimentos de Veracruz (AMBA)",
            needs: "Alimentos perecederos y no perecederos, granos básicos, arroz, frijol, leche, lácteos y fórmulas infantiles para comedores.",
            lat: 19.1834,
            lon: -96.1550,
        },
        NgoSeed {
            email: "asistencia@caritasveracruz.org",
            name: "Cáritas Diocesana de Veracruz",
            needs: "Medicamentos de cuadro básico, ropa en buen estado, calzado, cobijas y artículos de higiene personal familiar.",
            lat: 19.1982,
            lon: -96.1384,
        },
        NgoSeed {
            email: "ayuda@alberguelaesperanza.org",
            name: "Albergue Nocturno La Esperanza",
            needs: "Colchones, sábanas, cobijas abrigadoras, insumos de limpieza industrial y alimentos para cenas calientes.",
            lat: 19.1712,
            lon: -96.1310,
        },
        NgoSeed {
            email: "educacion@casadelninozamora.org",
            name: "Casa del Niño Manuel Gutiérrez Zamora",
            needs: "Material educativo, computadoras para estudio, calzado para menores, mochilas escolares y leche fortificada.",
            lat: 19.1905,
            lon: -96.1265,
        },
        NgoSeed {
            email: "atencion@asilosanantonio.org",
            name: "Asilo de Ancianos San Antonio de Padua",
            needs: "Pañales para adulto mayor, suplementos geriátricos, productos desinfectantes y leche deslactosada.",
            lat: 19.1880,
            lon: -96.1360,
        },
    ];

    let mut seeded_donations_count = 0;
    let mut all_created_donations = Vec::new();

    // 1. Sembrado de Empresas y Donaciones
    for comp in &companies {
        let user_row = sqlx::query!(
            r#"
            INSERT INTO users (email, password_hash, role)
            VALUES ($1, $2, 'empresa')
            ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
            RETURNING id
            "#,
            comp.email,
            default_password
        )
        .fetch_one(&state.db)
        .await
        .map_err(|_| internal_error())?;

        for (title, desc, qty) in &comp.donations {
            let existing_don = sqlx::query!(
                "SELECT id FROM donations WHERE user_id = $1 AND title = $2",
                user_row.id,
                title
            )
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

            let don_id = if let Some(d) = existing_don {
                d.id
            } else {
                let new_id = Uuid::new_v4();
                let _ = sqlx::query!(
                    r#"
                    INSERT INTO donations (id, user_id, title, description, quantity, status)
                    VALUES ($1, $2, $3, $4, $5, 'en_acopio')
                    "#,
                    new_id,
                    user_row.id,
                    title,
                    desc,
                    qty
                )
                .execute(&state.db)
                .await;
                seeded_donations_count += 1;
                new_id
            };

            all_created_donations.push((don_id, title.to_string(), desc.to_string()));
        }
    }

    // 2. Sembrado de ONGs e Indexación Vectorial
    let chroma = ChromaClient::new(None);
    let mut ngos_list = Vec::new();

    for ngo in &ngos {
        let user_row = sqlx::query!(
            r#"
            INSERT INTO users (email, password_hash, role)
            VALUES ($1, $2, 'ong')
            ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
            RETURNING id
            "#,
            ngo.email,
            default_password
        )
        .fetch_one(&state.db)
        .await
        .map_err(|_| internal_error())?;

        let ngo_row = sqlx::query!(
            "SELECT id FROM ngos WHERE name = $1 OR user_id = $2",
            ngo.name,
            user_row.id
        )
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

        let final_ngo_id = if let Some(rec) = ngo_row {
            let _ = sqlx::query!(
                r#"
                UPDATE ngos 
                SET needs_description = $1, latitude = $2, longitude = $3
                WHERE id = $4
                "#,
                ngo.needs,
                ngo.lat,
                ngo.lon,
                rec.id
            )
            .execute(&state.db)
            .await;
            rec.id
        } else {
            let new_id = Uuid::new_v4();
            let _ = sqlx::query!(
                r#"
                INSERT INTO ngos (id, user_id, name, needs_description, latitude, longitude, is_verified)
                VALUES ($1, $2, $3, $4, $5, $6, false)
                "#,
                new_id,
                user_row.id,
                ngo.name,
                ngo.needs,
                ngo.lat,
                ngo.lon
            )
            .execute(&state.db)
            .await;
            new_id
        };

        let _ = chroma.add_or_update_ngo(final_ngo_id, ngo.needs).await;
        ngos_list.push((
            final_ngo_id,
            ngo.name.to_string(),
            ngo.needs.to_string(),
            ngo.lat,
            ngo.lon,
        ));
    }

    // 3. Puntuación automática con DeepSeek-R1 (Groq)
    let groq = GroqClient::new();
    let mut ai_evaluations = Vec::new();

    for (_don_id, title, desc) in all_created_donations.iter().take(4) {
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
        message: "Ecosistema poblado y puntuado con Groq AI.".to_string(),
        companies_seeded: companies.len(),
        ngos_seeded: ngos.len(),
        donations_seeded: seeded_donations_count,
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
            "Banco de Alimentos de Veracruz (AMBA)",
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
