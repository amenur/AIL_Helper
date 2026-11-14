//! Catálogo de modelos embebido en el binario.
//!
//! El agente solo instala modelos y cuantizaciones de esta lista: la web no puede
//! pedirle descargar un repositorio arbitrario.

use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: String,
    pub hf_repo: String,
    pub quants: Vec<String>,
    pub max_context: u32,
}

const MODELS_JSON: &str = include_str!("../../shared/catalog/models.json");

pub fn models() -> &'static [Model] {
    static MODELS: OnceLock<Vec<Model>> = OnceLock::new();
    MODELS.get_or_init(|| serde_json::from_str(MODELS_JSON).expect("catálogo válido"))
}

pub fn find(id: &str) -> Option<&'static Model> {
    models().iter().find(|m| m.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_loads() {
        assert!(models().len() >= 10);
        let m = find("qwen3-8b").unwrap();
        assert_eq!(m.hf_repo, "Qwen/Qwen3-8B-GGUF");
        assert!(m.quants.contains(&"Q4_K_M".to_string()));
        assert!(find("../../etc/passwd").is_none());
    }
}
