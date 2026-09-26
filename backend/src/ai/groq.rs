use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroqEvaluation {
    pub compatibility_score: f64,
    pub reasoning: String,
    pub priority_level: String,
}

pub struct GroqClient {
    client: Client,
    api_key: Option<String>,
}

impl Default for GroqClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GroqClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            api_key: env::var("GROQ_API_KEY")
                .ok()
                .filter(|key| !key.trim().is_empty()),
        }
    }

    pub async fn evaluate_fit(
        &self,
        donation_title: &str,
        donation_desc: &str,
        ngo_name: &str,
        ngo_needs: &str,
        distance_km: f64,
    ) -> Option<GroqEvaluation> {
        let api_key = self.api_key.as_ref()?;

        let prompt = format!(
            "Evalúa la compatibilidad logística entre esta donación y la organización social receptora.\n\n\
            DONACIÓN DISPONIBLE:\n- Producto: {}\n- Especificaciones: {}\n\n\
            ORGANIZACIÓN SOCIAL:\n- Nombre: {}\n- Necesidades: {}\n- Distancia: {:.1} km\n\n\
            Responde ÚNICAMENTE con un JSON válido sin markdown ni texto extra:\n\
            {{\"compatibility_score\": <número entre 0 y 100>, \"reasoning\": \"<justificación de máximo 20 palabras>\", \"priority_level\": \"<Alta|Media|Baja>\"}}",
            donation_title, donation_desc, ngo_name, ngo_needs, distance_km
        );

        let body = serde_json::json!({
            "model": "deepseek-r1-distill-llama-70b",
            "messages": [
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.6
        });

        let res = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .ok()?;

        if !res.status().is_success() {
            return None;
        }

        let val: serde_json::Value = res.json().await.ok()?;
        let raw_content = val["choices"][0]["message"]["content"].as_str()?;

        // DeepSeek-R1 emite etiquetas <think>...</think>, las removemos
        let clean_str = if let Some(end_idx) = raw_content.find("</think>") {
            &raw_content[end_idx + 8..]
        } else {
            raw_content
        };

        // Extraer únicamente el bloque JSON {...}
        let start = clean_str.find('{')?;
        let end = clean_str.rfind('}')?;
        let json_slice = &clean_str[start..=end];

        serde_json::from_str::<GroqEvaluation>(json_slice).ok()
    }
}
