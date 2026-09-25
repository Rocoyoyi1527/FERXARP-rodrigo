//! Small, deterministic local feature vectors. No model download or external API is required.
use std::collections::BTreeSet;

pub const DIMENSIONS: usize = 256;

pub fn canonical_token(token: &str) -> &str {
    match token {
        "leche" | "lacteo" | "lacteos" | "lactea" | "lacteas" => "leche",
        "alimento" | "alimentos" | "comida" | "comidas" | "abarrote" | "abarrotes" | "despensa"
        | "despensas" => "alimento",
        "fruta" | "frutas" | "verdura" | "verduras" | "vegetal" | "vegetales" => "fruta",
        "computadora" | "computadoras" | "computo" | "laptop" | "laptops" | "servidor"
        | "servidores" => "computadora",
        "medicamento" | "medicamentos" | "medicina" | "medicinas" => "medicamento",
        "cobija" | "cobijas" | "manta" | "mantas" => "cobija",
        _ => token,
    }
}

pub fn terms(text: &str) -> BTreeSet<String> {
    let normalized: String = text
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect();

    normalized
        .split_whitespace()
        .filter(|word| word.len() > 2)
        .filter(|word| {
            !matches!(
                *word,
                "para"
                    | "con"
                    | "los"
                    | "las"
                    | "una"
                    | "unos"
                    | "del"
                    | "por"
                    | "que"
                    | "sus"
                    | "sin"
                    | "son"
                    | "cajas"
                    | "caja"
            )
        })
        .map(|word| canonical_token(word).to_string())
        .collect()
}

fn stable_hash(value: &str) -> u64 {
    value.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

pub fn embed(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0_f32; DIMENSIONS];
    for term in terms(text) {
        let index = (stable_hash(&term) as usize) % DIMENSIONS;
        vector[index] += 1.0;
    }
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_vector_is_deterministic_and_normalized() {
        assert_eq!(embed("Leche y lácteos"), embed("Leche y lácteos"));
        assert_eq!(embed("Leche"), embed("lácteos"));
        assert!((embed("leche").iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 0.0001);
    }
}
