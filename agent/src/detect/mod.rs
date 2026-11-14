//! Detección del hardware local.

mod gpu;
mod ram;

use crate::report::*;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

pub fn hardware_report() -> HardwareReport {
    let sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_memory(MemoryRefreshKind::nothing().with_ram())
            .with_cpu(CpuRefreshKind::nothing()),
    )
    .ok();

    let gpus = gpu::detect();
    let memory = memory(sys.as_ref());
    let cpu = cpu(sys.as_ref());

    HardwareReport {
        schema_version: SCHEMA_VERSION,
        agent_version: env!("CARGO_PKG_VERSION"),
        os: os(),
        cpu,
        memory,
        gpus,
        disk: disk(),
    }
}

fn os() -> OsInfo {
    let family = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" };
    let name = System::long_os_version()
        .or_else(|_| System::name())
        .unwrap_or_else(|_| family.to_string());
    OsInfo { family, name, arch }
}

fn cpu(sys: Option<&System>) -> CpuInfo {
    let logical = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let name = sys
        .and_then(|s| s.cpus().first())
        .map(|c| c.brand().trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "CPU desconocida".into());
    CpuInfo {
        name,
        physical_cores: System::physical_core_count().unwrap_or(logical),
        logical_cores: sys.map(|s| s.cpus().len()).filter(|&n| n > 0).unwrap_or(logical),
        flags: cpu_flags(),
    }
}

/// Extensiones de CPU que determinan qué build de llama.cpp conviene.
fn cpu_flags() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut flags = Vec::new();
    #[cfg(target_arch = "x86_64")]
    {
        macro_rules! check {
            ($($f:tt),*) => { $( if std::arch::is_x86_feature_detected!($f) { flags.push($f); } )* };
        }
        check!("avx", "avx2", "fma", "f16c", "avx512f", "avx512bw", "avx512vnni", "avxvnni");
    }
    #[cfg(target_arch = "aarch64")]
    {
        flags.push("neon");
        if std::arch::is_aarch64_feature_detected!("dotprod") {
            flags.push("dotprod");
        }
    }
    flags
}

fn memory(sys: Option<&System>) -> MemoryInfo {
    let (total, available) = sys.map(|s| (s.total_memory(), s.available_memory())).unwrap_or((0, 0));
    let modules = ram::detect();
    MemoryInfo {
        total_gb: bytes_to_gib(total),
        available_gb: bytes_to_gib(available),
        ram_type: modules.ram_type,
        speed_mts: modules.speed_mts,
        channels: modules.modules,
    }
}

/// Espacio libre donde se instalarán los modelos (carpeta de usuario).
fn disk() -> Option<DiskInfo> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })?;
    let home = std::path::PathBuf::from(home);
    let disks = Disks::new_with_refreshed_list().ok()?;
    // El punto de montaje más largo que contiene la carpeta de usuario.
    let disk = disks
        .iter()
        .filter(|d| home.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())?;
    Some(DiskInfo {
        install_path: home.join(".ail-helper").display().to_string(),
        free_gb: bytes_to_gb(disk.available_space()),
    })
}
