//! Arranque y parada de llama-server y Open WebUI.

use super::uv;
use crate::paths::Paths;
use anyhow::{Context, Result, bail};
use std::path::Path;
use std::time::Duration;
use tokio::process::{Child, Command};

pub const LLAMA_PORT: u16 = 8080;
pub const WEBUI_PORT: u16 = 3000;

#[derive(Default)]
pub struct Services {
    pub llama: Option<Child>,
    pub webui: Option<Child>,
}

impl Services {
    pub async fn stop(&mut self) {
        for child in [self.webui.take(), self.llama.take()].into_iter().flatten() {
            let mut child = child;
            let _ = child.kill().await;
        }
    }

    pub fn running(child: &mut Option<Child>) -> bool {
        match child {
            Some(c) => matches!(c.try_wait(), Ok(None)),
            None => false,
        }
    }
}

fn port_free(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn log_file(paths: &Paths, name: &str) -> Result<std::fs::File> {
    std::fs::create_dir_all(paths.logs())?;
    Ok(std::fs::File::create(paths.logs().join(format!("{name}.log")))?)
}

fn tail(paths: &Paths, name: &str) -> String {
    let text = std::fs::read_to_string(paths.logs().join(format!("{name}.log"))).unwrap_or_default();
    let lines: Vec<&str> = text.lines().rev().take(5).collect();
    lines.into_iter().rev().collect::<Vec<_>>().join("\n")
}

/// Espera a que `url` responda 200, vigilando que el proceso siga vivo.
async fn wait_healthy(child: &mut Child, url: &str, timeout: Duration, paths: &Paths, name: &str) -> Result<()> {
    let client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(2)).build()?;
    let start = std::time::Instant::now();
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            bail!("{name} se cerró ({status}). Últimas líneas del registro:\n{}", tail(paths, name));
        }
        if let Ok(res) = client.get(url).send().await
            && res.status().is_success()
        {
            return Ok(());
        }
        if start.elapsed() > timeout {
            bail!("{name} no respondió en {} s", timeout.as_secs());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

pub async fn start_llama(
    paths: &Paths,
    server: &Path,
    model: &Path,
    context: u32,
    gpu_layers: u32,
) -> Result<Child> {
    if !port_free(LLAMA_PORT) {
        bail!("el puerto {LLAMA_PORT} está ocupado: cierra el programa que lo usa e inténtalo de nuevo");
    }
    let log = log_file(paths, "llama-server")?;
    let mut cmd = Command::new(server);
    cmd.arg("-m")
        .arg(model)
        .args(["-c", &context.to_string()])
        .args(["-ngl", &gpu_layers.to_string()])
        .args(["--jinja", "--host", "127.0.0.1", "--port", &LLAMA_PORT.to_string()])
        .current_dir(server.parent().unwrap_or(Path::new(".")))
        .stdout(log.try_clone()?)
        .stderr(log)
        .kill_on_drop(true);
    super::hide_window(&mut cmd);
    let mut child = cmd.spawn().context("no se pudo arrancar llama-server")?;
    // Cargar un modelo grande puede tardar varios minutos.
    let url = format!("http://127.0.0.1:{LLAMA_PORT}/health");
    wait_healthy(&mut child, &url, Duration::from_secs(600), paths, "llama-server").await?;
    Ok(child)
}

pub async fn start_webui(paths: &Paths) -> Result<Child> {
    if !port_free(WEBUI_PORT) {
        bail!("el puerto {WEBUI_PORT} está ocupado: cierra el programa que lo usa e inténtalo de nuevo");
    }
    std::fs::create_dir_all(paths.webui_data())?;
    let log = log_file(paths, "open-webui")?;
    let mut cmd = Command::new(uv::webui_path(paths));
    cmd.args(["serve", "--host", "127.0.0.1", "--port", &WEBUI_PORT.to_string()])
        .envs(uv::uv_env(paths))
        .env("DATA_DIR", paths.webui_data())
        .env("OPENAI_API_BASE_URL", format!("http://127.0.0.1:{LLAMA_PORT}/v1"))
        .env("OPENAI_API_KEY", "none")
        .env("ENABLE_OLLAMA_API", "False")
        // Un solo usuario local: sin pantalla de registro ni contraseña.
        .env("WEBUI_AUTH", "False")
        .stdout(log.try_clone()?)
        .stderr(log)
        .kill_on_drop(true);
    super::hide_window(&mut cmd);
    let mut child = cmd.spawn().context("no se pudo arrancar Open WebUI")?;
    // El primer arranque descarga componentes y puede tardar unos minutos.
    let url = format!("http://127.0.0.1:{WEBUI_PORT}/health");
    wait_healthy(&mut child, &url, Duration::from_secs(900), paths, "open-webui").await?;
    Ok(child)
}
