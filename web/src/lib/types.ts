export interface Model {
  id: string;
  name: string;
  family: string;
  org: string;
  /** Parámetros totales, en miles de millones. */
  paramsB: number;
  /** Parámetros activos por token (solo MoE). */
  activeParamsB?: number;
  layers: number;
  kvHeads: number;
  headDim: number;
  maxContext: number;
  /** Factor de reducción de la caché KV (ventana deslizante, etc.). */
  kvCacheScale?: number;
  hfRepo: string;
  quants: string[];
  defaultQuant: string;
  tags: string[];
  description: string;
  license: string;
}

export interface Gpu {
  id: string;
  vendor: 'nvidia' | 'amd' | 'intel' | 'apple';
  name: string;
  vramGB: number;
  bandwidthGBs: number;
  fp16Tflops: number;
  backend: 'cuda' | 'rocm' | 'vulkan' | 'metal';
  laptop: boolean;
  unifiedMemory?: boolean;
}

export type RamType = 'ddr4' | 'ddr5' | 'lpddr5x' | 'unknown';

export interface HardwareProfile {
  /** De dónde vienen los datos. */
  source: 'browser' | 'manual' | 'agent';
  cpuThreads: number;
  ramGB: number;
  ramType: RamType;
  /** GPU elegida del catálogo, o null si solo CPU. */
  gpuId: string | null;
  /** VRAM real si difiere del catálogo (o la informa el agente). */
  vramGBOverride?: number | null;
  /** Texto bruto que dio el navegador/agente, para mostrarlo. */
  detectedGpuName?: string | null;
  /** Datos adicionales que solo da el agente local. */
  agent?: AgentDetails | null;
}

export interface AgentDetails {
  version: string;
  os: string;
  cpuName: string;
  diskFreeGB: number | null;
  ramSpeedMTs: number | null;
  driverVersion: string | null;
  cudaVersion: string | null;
}
