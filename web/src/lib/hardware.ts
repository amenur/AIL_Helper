import type { Gpu, HardwareProfile, RamType } from './types.ts';

/** Dirección del agente local (Rust). Solo escucha en 127.0.0.1. */
export const AGENT_URL = 'http://127.0.0.1:47800';

/** Descarga del agente (binarios publicados por el workflow de releases). */
export const AGENT_DOWNLOAD_URL = 'https://github.com/amenur/AIL_Helper/releases/latest';
/** Descarga directa del ejecutable de la última release. */
export const AGENT_WINDOWS_URL = `${AGENT_DOWNLOAD_URL}/download/ail-agent-windows-x64.exe`;
export const AGENT_LINUX_URL = `${AGENT_DOWNLOAD_URL}/download/ail-agent-linux-x64`;

function normalize(s: string): string {
  return s
    .toLowerCase()
    .replace(/\(tm\)|\(r\)|nvidia|geforce|amd|radeon|intel\(r\)|graphics/g, ' ')
    .replace(/[^a-z0-9]+/g, ' ')
    .trim();
}

/**
 * Busca en el catálogo la GPU que mejor encaja con un texto de renderer
 * (p. ej. "ANGLE (NVIDIA, NVIDIA GeForce RTX 4070 (0x00002786) Direct3D11 …)").
 * Gana la coincidencia más larga, para distinguir "4070" de "4070 Ti Super".
 */
export function matchGpu(renderer: string, gpus: Gpu[]): Gpu | null {
  const text = ` ${normalize(renderer)} `;
  let best: Gpu | null = null;
  let bestLen = 0;
  for (const gpu of gpus) {
    const name = normalize(gpu.name);
    if (text.includes(` ${name} `) && name.length > bestLen) {
      best = gpu;
      bestLen = name.length;
    }
  }
  return best;
}

/** Renderizado por software: el navegador no expone la GPU real. */
export function isSoftwareRenderer(renderer: string): boolean {
  return /swiftshader|llvmpipe|softpipe|basic render|microsoft basic/i.test(renderer);
}

export interface BrowserDetection {
  cpuThreads: number | null;
  /** navigator.deviceMemory: redondeado y con tope de 8 GB. */
  deviceMemoryGB: number | null;
  renderer: string | null;
  webgpu: { vendor: string; architecture: string; description: string } | null;
}

export async function detectInBrowser(): Promise<BrowserDetection> {
  const nav = navigator as Navigator & {
    deviceMemory?: number;
    gpu?: { requestAdapter(): Promise<{ info?: Record<string, string> } | null> };
  };
  const result: BrowserDetection = {
    cpuThreads: nav.hardwareConcurrency ?? null,
    deviceMemoryGB: nav.deviceMemory ?? null,
    renderer: null,
    webgpu: null,
  };

  try {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl2') ?? canvas.getContext('webgl');
    if (gl) {
      const ext = gl.getExtension('WEBGL_debug_renderer_info');
      result.renderer = ext
        ? String(gl.getParameter(ext.UNMASKED_RENDERER_WEBGL))
        : String(gl.getParameter(gl.RENDERER));
    }
  } catch {
    /* WebGL no disponible */
  }

  try {
    const adapter = await nav.gpu?.requestAdapter();
    if (adapter?.info) {
      result.webgpu = {
        vendor: adapter.info.vendor ?? '',
        architecture: adapter.info.architecture ?? '',
        description: adapter.info.description ?? '',
      };
    }
  } catch {
    /* WebGPU no disponible */
  }

  return result;
}

/** Subconjunto de shared/schema/hardware-report.schema.json que usa la web. */
export interface AgentHardwareReport {
  schemaVersion: 1;
  agentVersion: string;
  os: { family: string; name: string; arch: string };
  cpu: { name: string; logicalCores: number };
  memory: { totalGB: number; type?: string; speedMTs?: number | null };
  gpus: {
    vendor: string;
    name: string;
    vramGB: number;
    integrated?: boolean;
    driverVersion?: string | null;
    cudaVersion?: string | null;
  }[];
  disk?: { freeGB: number } | null;
}

export async function fetchAgentReport(timeoutMs = 800): Promise<AgentHardwareReport | null> {
  try {
    const res = await fetch(`${AGENT_URL}/v1/hardware`, {
      signal: AbortSignal.timeout(timeoutMs),
    });
    if (!res.ok) return null;
    const data = (await res.json()) as AgentHardwareReport;
    return data.schemaVersion === 1 ? data : null;
  } catch {
    return null;
  }
}

const RAM_TYPES: Record<string, RamType> = { ddr4: 'ddr4', ddr5: 'ddr5', lpddr5: 'lpddr5x', lpddr5x: 'lpddr5x' };

export function profileFromAgent(report: AgentHardwareReport, gpus: Gpu[]): HardwareProfile {
  // La GPU con más VRAM dedicada; las integradas no cuentan para llama.cpp.
  const main = [...report.gpus].filter((g) => !g.integrated && g.vramGB > 0).sort((a, b) => b.vramGB - a.vramGB)[0];
  const match = main ? matchGpu(main.name, gpus) : null;
  const ramType = RAM_TYPES[report.memory.type ?? ''] ?? 'unknown';
  return {
    source: 'agent',
    cpuThreads: report.cpu.logicalCores,
    ramGB: Math.round(report.memory.totalGB),
    ramType,
    gpuId: match?.id ?? null,
    vramGBOverride: main && !match?.unifiedMemory ? main.vramGB : null,
    detectedGpuName: main?.name ?? report.gpus[0]?.name ?? null,
    agent: {
      version: report.agentVersion,
      os: report.os.name,
      cpuName: report.cpu.name,
      diskFreeGB: report.disk?.freeGB ?? null,
      ramSpeedMTs: report.memory.speedMTs ?? null,
      driverVersion: main?.driverVersion ?? null,
      cudaVersion: main?.cudaVersion ?? null,
    },
  };
}

export function profileFromBrowser(det: BrowserDetection, gpus: Gpu[]): HardwareProfile {
  const renderer = [det.renderer, det.webgpu?.description].filter(Boolean).join(' ');
  const match = renderer ? matchGpu(renderer, gpus) : null;
  // deviceMemory tiene tope de 8: si dice 8, lo más probable es que haya más.
  const ram = det.deviceMemoryGB == null ? 16 : det.deviceMemoryGB >= 8 ? 16 : det.deviceMemoryGB;
  return {
    source: 'browser',
    cpuThreads: det.cpuThreads ?? 8,
    ramGB: ram,
    ramType: 'unknown',
    gpuId: match?.id ?? null,
    vramGBOverride: null,
    detectedGpuName: det.renderer ?? det.webgpu?.description ?? null,
    agent: null,
  };
}
