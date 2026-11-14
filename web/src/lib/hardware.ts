import type { Gpu, HardwareProfile, RamType } from './types.ts';

/** Dirección del agente local (Rust). Solo escucha en 127.0.0.1. */
export const AGENT_URL = 'http://127.0.0.1:47800';

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
  cpu: { name: string; logicalCores: number };
  memory: { totalGB: number; type?: string };
  gpus: { vendor: string; name: string; vramGB: number }[];
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

const RAM_TYPES: RamType[] = ['ddr4', 'ddr5', 'lpddr5x'];

export function profileFromAgent(report: AgentHardwareReport, gpus: Gpu[]): HardwareProfile {
  const main = [...report.gpus].sort((a, b) => b.vramGB - a.vramGB)[0];
  const match = main ? matchGpu(main.name, gpus) : null;
  const ramType = RAM_TYPES.find((t) => t === report.memory.type) ?? 'unknown';
  return {
    source: 'agent',
    cpuThreads: report.cpu.logicalCores,
    ramGB: Math.round(report.memory.totalGB),
    ramType,
    gpuId: match?.id ?? null,
    vramGBOverride: main && !match?.unifiedMemory ? main.vramGB : null,
    detectedGpuName: main?.name ?? null,
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
  };
}
