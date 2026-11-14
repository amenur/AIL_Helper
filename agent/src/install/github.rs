//! Releases de GitHub (llama.cpp y uv).

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    /// "sha256:<hex>" (GitHub lo calcula para cada asset).
    pub digest: Option<String>,
}

impl Asset {
    pub fn sha256(&self) -> Option<&str> {
        self.digest.as_deref().and_then(|d| d.strip_prefix("sha256:"))
    }
}

pub async fn latest_release(client: &reqwest::Client, repo: &str) -> Result<Release> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let res = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .with_context(|| format!("no se pudo consultar {url}"))?;
    if !res.status().is_success() {
        bail!("GitHub respondió {} al consultar {repo}", res.status());
    }
    Ok(res.json().await?)
}
