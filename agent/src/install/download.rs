//! Descargas con reanudación y verificación SHA-256.

use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("ail-helper-agent/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()
        .expect("cliente HTTP")
}

/// Descarga `url` en `dest`. Si existe `dest.part`, continúa donde se quedó (Range).
/// Si se indica `sha256`, se comprueba al terminar y, si no coincide, se borra.
/// `progress(descargado, total)` se llama a medida que llegan datos.
pub async fn download(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    sha256: Option<&str>,
    progress: impl Fn(u64, Option<u64>),
) -> Result<()> {
    if let Some(dir) = dest.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }
    let part = dest.with_file_name(format!(
        "{}.part",
        dest.file_name().context("ruta sin nombre de fichero")?.to_string_lossy()
    ));

    // Lo ya descargado entra en el hash antes de seguir.
    let mut hasher = Sha256::new();
    let mut have: u64 = 0;
    if let Ok(mut f) = tokio::fs::File::open(&part).await {
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = f.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            have += n as u64;
        }
    }

    let mut req = client.get(url);
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let res = req.send().await.with_context(|| format!("no se pudo conectar con {url}"))?;
    let status = res.status();
    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE && have > 0 {
        // El .part ya estaba completo.
    } else if !status.is_success() {
        bail!("{url} respondió {status}");
    }

    let resumed = status == reqwest::StatusCode::PARTIAL_CONTENT;
    if have > 0 && !resumed && status.is_success() {
        // El servidor no admite reanudar: empezamos de cero.
        hasher = Sha256::new();
        have = 0;
    }
    let total = res.content_length().map(|len| len + have);

    if status != reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(resumed)
            .truncate(!resumed)
            .open(&part)
            .await?;
        let mut stream = res.bytes_stream();
        let mut last_report = std::time::Instant::now();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("conexión interrumpida durante la descarga")?;
            file.write_all(&chunk).await?;
            hasher.update(&chunk);
            have += chunk.len() as u64;
            if last_report.elapsed().as_millis() >= 200 {
                progress(have, total);
                last_report = std::time::Instant::now();
            }
        }
        file.flush().await?;
    }
    progress(have, total.or(Some(have)));

    if let Some(expected) = sha256 {
        let got = hex::encode(hasher.finalize());
        if !got.eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&part).await;
            bail!("la suma SHA-256 de {} no coincide (esperada {expected}, obtenida {got})", dest.display());
        }
    }
    tokio::fs::rename(&part, dest).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, http::HeaderMap, response::IntoResponse, routing::get};

    const DATA: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

    /// Servidor de prueba que admite Range.
    async fn serve() -> String {
        async fn file(headers: HeaderMap) -> impl IntoResponse {
            if let Some(r) = headers.get("range").and_then(|v| v.to_str().ok()) {
                let start: usize = r.trim_start_matches("bytes=").trim_end_matches('-').parse().unwrap();
                return (axum::http::StatusCode::PARTIAL_CONTENT, DATA[start..].to_vec()).into_response();
            }
            DATA.to_vec().into_response()
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, Router::new().route("/f", get(file))).await.unwrap() });
        format!("http://{addr}/f")
    }

    fn sha(data: &[u8]) -> String {
        hex::encode(Sha256::digest(data))
    }

    #[tokio::test]
    async fn downloads_and_verifies() {
        let url = serve().await;
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("x.bin");
        download(&client(), &url, &dest, Some(&sha(DATA)), |_, _| {}).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), DATA);
    }

    #[tokio::test]
    async fn resumes_partial_download() {
        let url = serve().await;
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("x.bin");
        std::fs::write(dir.path().join("x.bin.part"), &DATA[..10]).unwrap();
        download(&client(), &url, &dest, Some(&sha(DATA)), |_, _| {}).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), DATA);
    }

    #[tokio::test]
    async fn rejects_wrong_checksum() {
        let url = serve().await;
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("x.bin");
        let err = download(&client(), &url, &dest, Some(&"0".repeat(64)), |_, _| {}).await.unwrap_err();
        assert!(err.to_string().contains("SHA-256"));
        assert!(!dest.exists());
        assert!(!dir.path().join("x.bin.part").exists());
    }
}
