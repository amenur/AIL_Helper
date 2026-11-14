//! Descarga de modelos GGUF de Hugging Face.

use super::download;
use crate::catalog::Model;
use crate::paths::Paths;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct RepoFile {
    #[serde(rename = "type")]
    pub kind: String,
    pub path: String,
    #[serde(default)]
    pub size: u64,
    pub lfs: Option<Lfs>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Lfs {
    /// SHA-256 del contenido.
    pub oid: String,
}

/// ¿El nombre lleva la cuantización como palabra completa? ("-Q4_K_M." o "-Q4_K_M-00001")
fn has_quant(file_name: &str, quant: &str) -> bool {
    let name = file_name.to_lowercase();
    let q = quant.to_lowercase();
    name.match_indices(&q).any(|(i, _)| {
        let before = name[..i].chars().last();
        let after = name[i + q.len()..].chars().next();
        matches!(before, Some('-' | '.' | '_' | '/')) && matches!(after, Some('-' | '.'))
    })
}

/// Ficheros GGUF de una cuantización (varios si el modelo está partido), en orden.
pub fn select_files(files: &[RepoFile], quant: &str) -> Vec<RepoFile> {
    let mut matches: Vec<RepoFile> = files
        .iter()
        .filter(|f| f.kind == "file")
        .filter(|f| {
            let lower = f.path.to_lowercase();
            lower.ends_with(".gguf") && !lower.contains("mmproj") && has_quant(&f.path, quant)
        })
        .cloned()
        .collect();
    // Si hay versión entera y partida, o en varias carpetas, preferimos la más cercana a la raíz.
    let min_depth = matches.iter().map(|f| f.path.matches('/').count()).min().unwrap_or(0);
    matches.retain(|f| f.path.matches('/').count() == min_depth);
    let single: Vec<RepoFile> = matches.iter().filter(|f| !f.path.contains("-of-")).cloned().collect();
    if let Some(first) = single.into_iter().min_by_key(|f| f.path.len()) {
        return vec![first];
    }
    matches.sort_by(|a, b| a.path.cmp(&b.path));
    matches
}

pub async fn list_files(client: &reqwest::Client, repo: &str) -> Result<Vec<RepoFile>> {
    let url = format!("https://huggingface.co/api/models/{repo}/tree/main?recursive=true");
    let res = client.get(&url).send().await.with_context(|| format!("no se pudo consultar {url}"))?;
    if !res.status().is_success() {
        bail!("Hugging Face respondió {} para {repo}", res.status());
    }
    Ok(res.json().await?)
}

/// Carpeta local del modelo: ~/.ail-helper/models/<org>__<repo>/
pub fn model_dir(paths: &Paths, model: &Model) -> PathBuf {
    paths.models().join(model.hf_repo.replace('/', "__"))
}

/// Primer fichero (el que se pasa a llama-server con -m) si ya está descargado completo.
pub fn installed(paths: &Paths, model: &Model, quant: &str) -> Option<PathBuf> {
    let marker = model_dir(paths, model).join(format!("{quant}.ok"));
    let first = std::fs::read_to_string(marker).ok()?;
    let path = model_dir(paths, model).join(first.trim());
    path.exists().then_some(path)
}

pub async fn download_model(
    client: &reqwest::Client,
    paths: &Paths,
    model: &Model,
    quant: &str,
    progress: &(dyn Fn(u64, Option<u64>, &str) + Sync),
) -> Result<PathBuf> {
    let files = select_files(&list_files(client, &model.hf_repo).await?, quant);
    if files.is_empty() {
        bail!("{} no tiene ficheros GGUF {quant}", model.hf_repo);
    }
    let total: u64 = files.iter().map(|f| f.size).sum();
    let dir = model_dir(paths, model);
    let mut done_before = 0u64;
    let mut local = Vec::new();
    for f in &files {
        let name = f.path.rsplit('/').next().unwrap_or(&f.path);
        let dest = dir.join(name);
        let ok_size = std::fs::metadata(&dest).map(|m| m.len() == f.size).unwrap_or(false);
        if !ok_size {
            let url = format!("https://huggingface.co/{}/resolve/main/{}", model.hf_repo, f.path);
            let sha = f.lfs.as_ref().map(|l| l.oid.as_str());
            let label = name.to_string();
            download::download(client, &url, &dest, sha, |d, _| progress(done_before + d, Some(total), &label)).await?;
        }
        done_before += f.size;
        local.push(name.to_string());
    }
    std::fs::write(dir.join(format!("{quant}.ok")), &local[0])?;
    Ok(dir.join(&local[0]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(path: &str) -> RepoFile {
        RepoFile { kind: "file".into(), path: path.into(), size: 1, lfs: None }
    }

    #[test]
    fn picks_single_file() {
        let files = [f("Qwen3-8B-Q4_K_M.gguf"), f("Qwen3-8B-Q5_K_M.gguf"), f("Qwen3-8B-Q8_0.gguf"), f("README.md")];
        let s = select_files(&files, "Q4_K_M");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].path, "Qwen3-8B-Q4_K_M.gguf");
    }

    #[test]
    fn does_not_confuse_similar_quants() {
        let files = [f("m-Q4_K_M.gguf"), f("m-Q4_K_S.gguf"), f("m-IQ4_XS.gguf"), f("m-Q4_0.gguf")];
        assert_eq!(select_files(&files, "Q4_0")[0].path, "m-Q4_0.gguf");
        assert_eq!(select_files(&files, "Q4_K_M")[0].path, "m-Q4_K_M.gguf");
    }

    #[test]
    fn ignores_mmproj_and_lowercase_names() {
        let files = [f("mmproj-model-f16.gguf"), f("gpt-oss-20b-mxfp4.gguf")];
        assert_eq!(select_files(&files, "MXFP4")[0].path, "gpt-oss-20b-mxfp4.gguf");
        let files = [f("gemma-3-12b-it-Q4_K_M.gguf"), f("mmproj-gemma-Q4_K_M.gguf")];
        assert_eq!(select_files(&files, "Q4_K_M")[0].path, "gemma-3-12b-it-Q4_K_M.gguf");
    }

    #[test]
    fn split_files_in_folder() {
        let files = [
            f("Q6_K/Llama-3.3-70B-Instruct-Q6_K-00002-of-00002.gguf"),
            f("Q6_K/Llama-3.3-70B-Instruct-Q6_K-00001-of-00002.gguf"),
            f("Llama-3.3-70B-Instruct-Q4_K_M.gguf"),
        ];
        let s = select_files(&files, "Q6_K");
        assert_eq!(s.len(), 2);
        assert!(s[0].path.contains("00001"));
    }
}
