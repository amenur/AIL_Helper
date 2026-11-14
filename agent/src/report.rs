//! Tipos del informe de hardware.
//! Deben coincidir con `shared/schema/hardware-report.schema.json`.

use serde::Serialize;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareReport {
    pub schema_version: u32,
    pub agent_version: &'static str,
    pub os: OsInfo,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpus: Vec<GpuInfo>,
    pub disk: Option<DiskInfo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsInfo {
    pub family: &'static str,
    pub name: String,
    pub arch: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub name: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub flags: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RamType {
    Ddr4,
    Ddr5,
    Lpddr5,
    #[allow(dead_code)] // SMBIOS no distingue LPDDR5X; se mantiene por el esquema.
    Lpddr5x,
    #[default]
    Unknown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    #[serde(rename = "totalGB")]
    pub total_gb: f64,
    #[serde(rename = "availableGB")]
    pub available_gb: f64,
    #[serde(rename = "type")]
    pub ram_type: RamType,
    #[serde(rename = "speedMTs")]
    pub speed_mts: Option<u32>,
    pub channels: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Vendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Other,
}

impl Vendor {
    pub fn from_pci_id(id: u32) -> Self {
        match id {
            0x10de => Vendor::Nvidia,
            0x1002 | 0x1022 => Vendor::Amd,
            0x8086 => Vendor::Intel,
            0x106b => Vendor::Apple,
            _ => Vendor::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    Cuda,
    Rocm,
    Vulkan,
    Metal,
    Sycl,
    Cpu,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub vendor: Vendor,
    pub name: String,
    #[serde(rename = "vramGB")]
    pub vram_gb: f64,
    #[serde(rename = "vramFreeGB")]
    pub vram_free_gb: Option<f64>,
    pub driver_version: Option<String>,
    pub cuda_version: Option<String>,
    pub compute_capability: Option<String>,
    /// GPU integrada (sin VRAM dedicada significativa).
    pub integrated: bool,
    pub backends: Vec<Backend>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub install_path: String,
    #[serde(rename = "freeGB")]
    pub free_gb: f64,
}

pub fn bytes_to_gb(bytes: u64) -> f64 {
    (bytes as f64 / 1e9 * 10.0).round() / 10.0
}

/// GiB redondeados: así reportan la VRAM los fabricantes (12 GB = 12 GiB).
pub fn bytes_to_gib(bytes: u64) -> f64 {
    (bytes as f64 / (1u64 << 30) as f64 * 10.0).round() / 10.0
}
