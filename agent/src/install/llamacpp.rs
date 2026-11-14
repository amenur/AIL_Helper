//! Elección e instalación del build de llama.cpp.

use super::{archive, download, github};
use crate::paths::{Paths, exe};
use crate::report::{GpuInfo, Vendor};
use anyhow::{Result, bail};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LlamaBackend {
    Cuda,
    Hip,
    Vulkan,
    Cpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Linux,
}

impl Os {
    pub fn current() -> Self {
        if cfg!(windows) { Os::Windows } else { Os::Linux }
    }
}

/// Backends a probar, en orden de preferencia, según la GPU principal.
pub fn preferences(os: Os, gpu: Option<&GpuInfo>, vulkan_available: bool) -> Vec<LlamaBackend> {
    use LlamaBackend::*;
    let Some(gpu) = gpu.filter(|g| !g.integrated && g.vram_gb > 0.0) else {
        return vec![Cpu];
    };
    let mut prefs = match (os, gpu.vendor) {
        (Os::Windows, Vendor::Nvidia) if gpu.cuda_version.is_some() => vec![Cuda, Vulkan],
        (Os::Windows, Vendor::Amd) => vec![Hip, Vulkan],
        (Os::Windows, _) => vec![Vulkan],
        // En Linux no hay binario CUDA oficial; Vulkan funciona con NVIDIA, AMD e Intel.
        (Os::Linux, _) if vulkan_available => vec![Vulkan],
        (Os::Linux, _) => vec![],
    };
    prefs.push(Cpu);
    prefs
}

/// ¿Está el cargador de Vulkan? Sin él, el build Vulkan de Linux no arranca.
pub fn vulkan_available() -> bool {
    if cfg!(windows) {
        return true; // viene con los drivers de GPU
    }
    [
        "/usr/lib/x86_64-linux-gnu/libvulkan.so.1",
        "/usr/lib64/libvulkan.so.1",
        "/usr/lib/libvulkan.so.1",
    ]
    .iter()
    .any(|p| std::path::Path::new(p).exists())
}

fn parse_version(s: &str) -> Option<(u32, u32)> {
    let (a, b) = s.split_once('.')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// Nombres de los assets a descargar para un backend, o None si la release no lo tiene.
/// `cuda_driver`: versión máxima de CUDA que admite el driver instalado.
pub fn select_assets(names: &[String], os: Os, backend: LlamaBackend, cuda_driver: Option<(u32, u32)>) -> Option<Vec<String>> {
    let is_archive = |n: &str| n.ends_with(".zip") || n.ends_with(".tar.gz");
    let find = |suffix: &str| {
        names
            .iter()
            .find(|n| n.starts_with("llama-") && n.contains(suffix) && is_archive(n))
            .cloned()
    };
    match (os, backend) {
        (Os::Windows, LlamaBackend::Cuda) => {
            // llama-bXXXX-bin-win-cuda-12.4-x64.zip + cudart-llama-bin-win-cuda-12.4-x64.zip
            let mut builds: Vec<((u32, u32), String)> = names
                .iter()
                .filter(|n| n.starts_with("llama-") && n.ends_with("-x64.zip"))
                .filter_map(|n| {
                    let ver = n.split("-bin-win-cuda-").nth(1)?.strip_suffix("-x64.zip")?;
                    Some((parse_version(ver)?, n.clone()))
                })
                .filter(|(v, _)| cuda_driver.is_none_or(|d| *v <= d))
                .collect();
            builds.sort();
            let (ver, build) = builds.pop()?;
            let cudart = format!("cudart-llama-bin-win-cuda-{}.{}-x64.zip", ver.0, ver.1);
            let mut out = vec![build];
            if names.contains(&cudart) {
                out.push(cudart);
            }
            Some(out)
        }
        (Os::Windows, LlamaBackend::Hip) => find("-bin-win-hip").map(|n| vec![n]),
        (Os::Windows, LlamaBackend::Vulkan) => find("-bin-win-vulkan-x64").map(|n| vec![n]),
        (Os::Windows, LlamaBackend::Cpu) => find("-bin-win-cpu-x64").map(|n| vec![n]),
        (Os::Linux, LlamaBackend::Vulkan) => find("-bin-ubuntu-vulkan-x64").map(|n| vec![n]),
        (Os::Linux, LlamaBackend::Cpu) => find("-bin-ubuntu-x64").map(|n| vec![n]),
        (Os::Linux, _) => None,
    }
}

pub struct Installed {
    pub tag: String,
    pub backend: LlamaBackend,
}

fn current_file(paths: &Paths) -> std::path::PathBuf {
    paths.llama().join("current")
}

/// Versión instalada, si la hay y su llama-server existe.
pub fn installed(paths: &Paths) -> Option<(String, std::path::PathBuf)> {
    let tag_backend = std::fs::read_to_string(current_file(paths)).ok()?;
    let dir = paths.llama().join(tag_backend.trim());
    let server = archive::find_file(&dir, &exe("llama-server"))?;
    Some((tag_backend.trim().to_string(), server))
}

pub async fn install(
    client: &reqwest::Client,
    paths: &Paths,
    prefs: &[LlamaBackend],
    cuda_driver: Option<(u32, u32)>,
    progress: &(dyn Fn(u64, Option<u64>, &str) + Sync),
) -> Result<Installed> {
    let release = github::latest_release(client, "ggml-org/llama.cpp").await?;
    let names: Vec<String> = release.assets.iter().map(|a| a.name.clone()).collect();
    let os = Os::current();
    let Some((backend, assets)) = prefs
        .iter()
        .find_map(|b| select_assets(&names, os, *b, cuda_driver).map(|a| (*b, a)))
    else {
        bail!("la release {} de llama.cpp no tiene un binario para este sistema", release.tag_name);
    };

    let key = format!("{}-{}", release.tag_name, format!("{backend:?}").to_lowercase());
    let dir = paths.llama().join(&key);
    if archive::find_file(&dir, &exe("llama-server")).is_none() {
        let downloads = paths.root.join("downloads");
        for name in &assets {
            let asset = release.assets.iter().find(|a| &a.name == name).expect("asset elegido");
            let file = downloads.join(name);
            let label = name.to_string();
            download::download(client, &asset.browser_download_url, &file, asset.sha256(), |d, t| progress(d, t, &label)).await?;
            archive::extract(&file, &dir)?;
            let _ = std::fs::remove_file(&file);
        }
    }
    let server = archive::find_file(&dir, &exe("llama-server"))
        .ok_or_else(|| anyhow::anyhow!("el paquete de llama.cpp no contiene llama-server"))?;
    archive::make_executable(&server)?;
    std::fs::write(current_file(paths), &key)?;
    Ok(Installed { tag: release.tag_name, backend })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Backend;

    fn names() -> Vec<String> {
        [
            "cudart-llama-bin-win-cuda-12.4-x64.zip",
            "cudart-llama-bin-win-cuda-13.0-x64.zip",
            "llama-b7100-bin-macos-arm64.zip",
            "llama-b7100-bin-ubuntu-vulkan-x64.zip",
            "llama-b7100-bin-ubuntu-x64.zip",
            "llama-b7100-bin-win-cpu-arm64.zip",
            "llama-b7100-bin-win-cpu-x64.zip",
            "llama-b7100-bin-win-cuda-12.4-x64.zip",
            "llama-b7100-bin-win-cuda-13.0-x64.zip",
            "llama-b7100-bin-win-hip-radeon-x64.zip",
            "llama-b7100-bin-win-vulkan-x64.zip",
        ]
        .map(String::from)
        .to_vec()
    }

    fn gpu(vendor: Vendor, cuda: Option<&str>) -> GpuInfo {
        GpuInfo {
            vendor,
            name: "x".into(),
            vram_gb: 12.0,
            vram_free_gb: None,
            driver_version: None,
            cuda_version: cuda.map(String::from),
            compute_capability: None,
            integrated: false,
            backends: vec![Backend::Cpu],
        }
    }

    #[test]
    fn cuda_picks_newest_supported_by_driver() {
        let a = select_assets(&names(), Os::Windows, LlamaBackend::Cuda, Some((12, 8))).unwrap();
        assert_eq!(a, vec!["llama-b7100-bin-win-cuda-12.4-x64.zip", "cudart-llama-bin-win-cuda-12.4-x64.zip"]);
        let a = select_assets(&names(), Os::Windows, LlamaBackend::Cuda, Some((13, 0))).unwrap();
        assert_eq!(a[0], "llama-b7100-bin-win-cuda-13.0-x64.zip");
        assert!(select_assets(&names(), Os::Windows, LlamaBackend::Cuda, Some((11, 8))).is_none());
    }

    #[test]
    fn other_backends() {
        let n = names();
        assert_eq!(select_assets(&n, Os::Windows, LlamaBackend::Hip, None).unwrap()[0], "llama-b7100-bin-win-hip-radeon-x64.zip");
        assert_eq!(select_assets(&n, Os::Windows, LlamaBackend::Cpu, None).unwrap()[0], "llama-b7100-bin-win-cpu-x64.zip");
        assert_eq!(select_assets(&n, Os::Linux, LlamaBackend::Vulkan, None).unwrap()[0], "llama-b7100-bin-ubuntu-vulkan-x64.zip");
        assert_eq!(select_assets(&n, Os::Linux, LlamaBackend::Cpu, None).unwrap()[0], "llama-b7100-bin-ubuntu-x64.zip");
        assert!(select_assets(&n, Os::Linux, LlamaBackend::Cuda, None).is_none());
    }

    #[test]
    fn backend_preferences() {
        use LlamaBackend::*;
        assert_eq!(preferences(Os::Windows, Some(&gpu(Vendor::Nvidia, Some("12.8"))), true), vec![Cuda, Vulkan, Cpu]);
        assert_eq!(preferences(Os::Windows, Some(&gpu(Vendor::Amd, None)), true), vec![Hip, Vulkan, Cpu]);
        assert_eq!(preferences(Os::Linux, Some(&gpu(Vendor::Nvidia, Some("12.8"))), true), vec![Vulkan, Cpu]);
        assert_eq!(preferences(Os::Linux, Some(&gpu(Vendor::Amd, None)), false), vec![Cpu]);
        assert_eq!(preferences(Os::Windows, None, true), vec![Cpu]);
    }
}
