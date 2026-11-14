//! Detección de GPUs.
//!
//! 1. NVIDIA: NVML (se carga dinámicamente con el driver; si no hay driver, se omite).
//! 2. Resto de fabricantes:
//!    - Windows: DXGI (nombre y VRAM dedicada).
//!    - Linux: /sys/class/drm (fabricante PCI, VRAM de amdgpu) + lspci para el nombre.

use crate::report::*;

/// Por debajo de esto consideramos la GPU integrada (memoria "dedicada" simbólica).
const INTEGRATED_THRESHOLD_GB: f64 = 2.0;

pub fn detect() -> Vec<GpuInfo> {
    let mut gpus = nvidia();
    let have_nvml = !gpus.is_empty();
    for gpu in platform() {
        if have_nvml && gpu.vendor == Vendor::Nvidia {
            continue;
        }
        gpus.push(gpu);
    }
    // Primero la de más VRAM: es la que usará llama.cpp.
    gpus.sort_by(|a, b| b.vram_gb.total_cmp(&a.vram_gb));
    gpus
}

pub fn backends_for(vendor: Vendor, integrated: bool, has_cuda: bool) -> Vec<Backend> {
    let mut b = match vendor {
        Vendor::Nvidia if has_cuda => vec![Backend::Cuda, Backend::Vulkan],
        Vendor::Nvidia => vec![Backend::Vulkan],
        Vendor::Amd if integrated => vec![Backend::Vulkan],
        Vendor::Amd => vec![Backend::Rocm, Backend::Vulkan],
        Vendor::Intel => vec![Backend::Vulkan, Backend::Sycl],
        Vendor::Apple => vec![Backend::Metal],
        Vendor::Other => vec![Backend::Vulkan],
    };
    b.push(Backend::Cpu);
    b
}

fn nvidia() -> Vec<GpuInfo> {
    let Ok(nvml) = nvml_wrapper::Nvml::init() else {
        return Vec::new();
    };
    let driver = nvml.sys_driver_version().ok();
    let cuda = nvml.sys_cuda_driver_version().ok().map(|v| {
        format!(
            "{}.{}",
            nvml_wrapper::cuda_driver_version_major(v),
            nvml_wrapper::cuda_driver_version_minor(v)
        )
    });
    let count = nvml.device_count().unwrap_or(0);
    (0..count)
        .filter_map(|i| {
            let dev = nvml.device_by_index(i).ok()?;
            let mem = dev.memory_info().ok()?;
            let cc = dev.cuda_compute_capability().ok().map(|c| format!("{}.{}", c.major, c.minor));
            Some(GpuInfo {
                vendor: Vendor::Nvidia,
                name: dev.name().unwrap_or_else(|_| "NVIDIA GPU".into()),
                vram_gb: bytes_to_gib(mem.total),
                vram_free_gb: Some(bytes_to_gib(mem.free)),
                driver_version: driver.clone(),
                cuda_version: cuda.clone(),
                compute_capability: cc,
                integrated: false,
                backends: backends_for(Vendor::Nvidia, false, true),
            })
        })
        .collect()
}

fn basic(vendor: Vendor, name: String, vram_bytes: u64) -> GpuInfo {
    let vram_gb = bytes_to_gib(vram_bytes);
    let integrated = vram_gb < INTEGRATED_THRESHOLD_GB;
    GpuInfo {
        vendor,
        name,
        vram_gb: if integrated { 0.0 } else { vram_gb },
        vram_free_gb: None,
        driver_version: None,
        cuda_version: None,
        compute_capability: None,
        integrated,
        backends: backends_for(vendor, integrated, false),
    }
}

#[cfg(windows)]
fn platform() -> Vec<GpuInfo> {
    use windows::Win32::Graphics::Dxgi::*;

    let mut out = Vec::new();
    // SAFETY: llamadas COM de solo lectura; los punteros los gestiona el crate `windows`.
    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else { return out };
        let mut i = 0;
        while let Ok(adapter) = factory.EnumAdapters1(i) {
            i += 1;
            let Ok(desc) = adapter.GetDesc1() else { continue };
            if desc.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0 {
                continue; // "Microsoft Basic Render Driver"
            }
            let len = desc.Description.iter().position(|&c| c == 0).unwrap_or(desc.Description.len());
            let name = String::from_utf16_lossy(&desc.Description[..len]);
            out.push(basic(Vendor::from_pci_id(desc.VendorId), name, desc.DedicatedVideoMemory as u64));
        }
    }
    out
}

#[cfg(target_os = "linux")]
fn platform() -> Vec<GpuInfo> {
    use std::fs;

    let Ok(entries) = fs::read_dir("/sys/class/drm") else { return Vec::new() };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let card = entry.file_name().to_string_lossy().into_owned();
        // Solo "cardN", no los conectores ("card0-DP-1").
        if !card.strip_prefix("card").is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
            continue;
        }
        let dev = entry.path().join("device");
        let read_hex = |f: &str| {
            fs::read_to_string(dev.join(f))
                .ok()
                .and_then(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok())
        };
        let Some(vendor_id) = read_hex("vendor") else { continue };
        let vendor = Vendor::from_pci_id(vendor_id);
        let vram = fs::read_to_string(dev.join("mem_info_vram_total"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);
        let pci_addr = fs::canonicalize(&dev)
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
        let name = pci_addr
            .as_deref()
            .and_then(lspci_name)
            .or_else(|| fs::read_to_string(dev.join("product_name")).ok().map(|s| s.trim().to_string()))
            .unwrap_or_else(|| {
                let device = read_hex("device").unwrap_or(0);
                format!("{vendor:?} GPU (0x{device:04x})")
            });
        // Las NVIDIA sin NVML no informan VRAM por sysfs: quedarán como 0.
        out.push(basic(vendor, name, vram));
    }
    out
}

/// Nombre comercial vía `lspci -vmm -s <dirección>` (argumentos fijos).
#[cfg(target_os = "linux")]
fn lspci_name(addr: &str) -> Option<String> {
    if !addr.chars().all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.') {
        return None;
    }
    let out = std::process::Command::new("lspci").args(["-vmm", "-s", addr]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let field = |key: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(key))
            .map(|v| v.trim().to_string())
    };
    let device = field("Device:")?;
    // Muchas veces el nombre comercial va entre corchetes: "Navi 31 [Radeon RX 7900 XT/7900 XTX]".
    let pretty = device
        .find('[')
        .and_then(|s| device[s + 1..].find(']').map(|e| device[s + 1..s + 1 + e].to_string()))
        .unwrap_or(device);
    Some(pretty)
}

#[cfg(not(any(windows, target_os = "linux")))]
fn platform() -> Vec<GpuInfo> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrated_gpu_has_no_vram() {
        let g = basic(Vendor::Intel, "Intel UHD Graphics 770".into(), 128 << 20);
        assert!(g.integrated);
        assert_eq!(g.vram_gb, 0.0);
        assert_eq!(g.backends, vec![Backend::Vulkan, Backend::Sycl, Backend::Cpu]);
    }

    #[test]
    fn dedicated_amd_gpu() {
        let g = basic(Vendor::Amd, "Radeon RX 7900 XTX".into(), 24 << 30);
        assert!(!g.integrated);
        assert_eq!(g.vram_gb, 24.0);
        assert_eq!(g.backends[0], Backend::Rocm);
    }
}
