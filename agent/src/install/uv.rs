//! uv (gestor de Python) y Open WebUI, aislados dentro de ~/.ail-helper.

use super::{archive, download, github};
use crate::paths::{Paths, exe};
use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};

fn uv_asset_name() -> &'static str {
    if cfg!(windows) { "uv-x86_64-pc-windows-msvc.zip" } else { "uv-x86_64-unknown-linux-gnu.tar.gz" }
}

pub fn uv_path(paths: &Paths) -> PathBuf {
    paths.bin().join(exe("uv"))
}

pub fn webui_path(paths: &Paths) -> PathBuf {
    paths.bin().join(exe("open-webui"))
}

pub async fn install_uv(client: &reqwest::Client, paths: &Paths, progress: &(dyn Fn(u64, Option<u64>, &str) + Sync)) -> Result<()> {
    let release = github::latest_release(client, "astral-sh/uv").await?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == uv_asset_name())
        .with_context(|| format!("la release {} de uv no incluye {}", release.tag_name, uv_asset_name()))?;
    let file = paths.root.join("downloads").join(&asset.name);
    download::download(client, &asset.browser_download_url, &file, asset.sha256(), |d, t| progress(d, t, &asset.name)).await?;
    let tmp = paths.root.join("downloads").join("uv-extract");
    let _ = std::fs::remove_dir_all(&tmp);
    archive::extract(&file, &tmp)?;
    std::fs::create_dir_all(paths.bin())?;
    for name in [exe("uv"), exe("uvx")] {
        if let Some(src) = archive::find_file(&tmp, &name) {
            let dest = paths.bin().join(&name);
            std::fs::copy(&src, &dest)?;
            archive::make_executable(&dest)?;
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_file(&file);
    if !uv_path(paths).exists() {
        bail!("el paquete de uv no contiene el ejecutable");
    }
    Ok(())
}

/// Variables para que uv trabaje solo dentro de ~/.ail-helper.
pub fn uv_env(paths: &Paths) -> Vec<(&'static str, PathBuf)> {
    vec![
        ("UV_TOOL_DIR", paths.uv_tools()),
        ("UV_TOOL_BIN_DIR", paths.bin()),
        ("UV_PYTHON_INSTALL_DIR", paths.uv_python()),
        ("UV_CACHE_DIR", paths.uv_cache()),
    ]
}

/// `uv tool install --python 3.11 open-webui`. Cada línea de salida se pasa a `log`.
pub async fn install_webui(paths: &Paths, log: &(dyn Fn(&str) + Sync)) -> Result<()> {
    let mut cmd = tokio::process::Command::new(uv_path(paths));
    cmd.args(["tool", "install", "--python", "3.11", "open-webui"])
        .envs(uv_env(paths))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    super::hide_window(&mut cmd);
    let mut child = cmd.spawn().context("no se pudo ejecutar uv")?;
    let stderr = child.stderr.take().expect("stderr");
    let stdout = child.stdout.take().expect("stdout");
    let mut err_lines = BufReader::new(stderr).lines();
    let mut out_lines = BufReader::new(stdout).lines();
    let mut last = String::new();
    loop {
        tokio::select! {
            line = err_lines.next_line() => match line? {
                Some(l) => { log(&l); last = l; }
                None => break,
            },
            line = out_lines.next_line() => if let Some(l) = line? { log(&l); },
        }
    }
    let status = child.wait().await?;
    if !status.success() || !webui_path(paths).exists() {
        bail!("la instalación de Open WebUI falló: {last}");
    }
    Ok(())
}
