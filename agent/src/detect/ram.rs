//! Tipo y velocidad de los módulos de RAM (SMBIOS tipo 17).
//!
//! - Windows: WMI (`Win32_PhysicalMemory`) vía PowerShell, sin privilegios.
//! - Linux: tablas DMI en /sys/firmware/dmi; normalmente requieren root,
//!   así que sin privilegios se informa `unknown`.

use crate::report::RamType;

#[derive(Debug, Default)]
pub struct RamModules {
    pub ram_type: RamType,
    pub speed_mts: Option<u32>,
    pub modules: Option<u32>,
}

/// Códigos SMBIOS de "Memory Type".
pub fn smbios_type(code: u32) -> RamType {
    match code {
        0x1A => RamType::Ddr4,
        0x22 => RamType::Ddr5,
        0x23 => RamType::Lpddr5,
        _ => RamType::Unknown,
    }
}

/// Combina los módulos: el tipo más frecuente y la velocidad mínima.
fn summarize(entries: &[(u32, u32)]) -> RamModules {
    if entries.is_empty() {
        return RamModules::default();
    }
    let ram_type = entries.iter().map(|(t, _)| smbios_type(*t)).find(|t| *t != RamType::Unknown).unwrap_or_default();
    let speed = entries.iter().map(|(_, s)| *s).filter(|s| *s > 0).min();
    RamModules { ram_type, speed_mts: speed, modules: Some(entries.len() as u32) }
}

#[cfg(target_os = "linux")]
pub fn detect() -> RamModules {
    let Ok(dir) = std::fs::read_dir("/sys/firmware/dmi/entries") else {
        return RamModules::default();
    };
    let mut entries = Vec::new();
    for entry in dir.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("17-") {
            continue;
        }
        let Ok(raw) = std::fs::read(entry.path().join("raw")) else { continue };
        if let Some(e) = parse_smbios17(&raw) {
            entries.push(e);
        }
    }
    summarize(&entries)
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
/// Extrae (tipo, velocidad MT/s) de una estructura SMBIOS tipo 17.
/// Devuelve None si el hueco está vacío (tamaño 0).
pub fn parse_smbios17(raw: &[u8]) -> Option<(u32, u32)> {
    if raw.len() < 0x17 || raw[0] != 17 {
        return None;
    }
    let size = u16::from_le_bytes([raw[0x0C], raw[0x0D]]);
    if size == 0 {
        return None;
    }
    let ty = raw[0x12] as u32;
    let speed = u16::from_le_bytes([raw[0x15], raw[0x16]]) as u32;
    Some((ty, speed))
}

#[cfg(windows)]
pub fn detect() -> RamModules {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // Comando fijo, sin datos de entrada externos.
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_PhysicalMemory | ForEach-Object { \"$($_.SMBIOSMemoryType) $($_.ConfiguredClockSpeed)\" }",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let Ok(out) = out else { return RamModules::default() };
    let entries: Vec<(u32, u32)> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace().map(|n| n.parse::<u32>().ok());
            Some((it.next()??, it.next().flatten().unwrap_or(0)))
        })
        .collect();
    summarize(&entries)
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn detect() -> RamModules {
    RamModules::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smbios17_ddr5() {
        let mut raw = vec![0u8; 0x28];
        raw[0] = 17;
        raw[0x0C] = 0x00;
        raw[0x0D] = 0x40; // 16 GB
        raw[0x12] = 0x22; // DDR5
        raw[0x15..0x17].copy_from_slice(&5600u16.to_le_bytes());
        assert_eq!(parse_smbios17(&raw), Some((0x22, 5600)));
        let m = summarize(&[(0x22, 5600), (0x22, 5200)]);
        assert_eq!(m.ram_type, RamType::Ddr5);
        assert_eq!(m.speed_mts, Some(5200));
        assert_eq!(m.modules, Some(2));
    }

    #[test]
    fn smbios17_empty_slot() {
        let mut raw = vec![0u8; 0x28];
        raw[0] = 17;
        assert_eq!(parse_smbios17(&raw), None);
    }
}
