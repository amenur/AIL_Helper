/**
 * Motor de estimación de rendimiento para llama.cpp.
 *
 * Modelo simplificado:
 *  - Generación (decode) limitada por ancho de banda de memoria:
 *      tiempo/token ≈ bytes leídos por token / ancho de banda efectivo
 *    En descarga parcial (parte en GPU, parte en RAM) se suman los tiempos
 *    de cada parte, por lo que la porción en CPU suele dominar.
 *  - Procesado del prompt (prefill) limitado por cómputo:
 *      tokens/s ≈ FLOPS efectivos / (2 · parámetros activos)
 *
 * Son estimaciones orientativas (±30 %), no benchmarks.
 */
import type { Gpu, HardwareProfile, Model, RamType } from './types.ts';

export interface QuantInfo {
  bpw: number;
  quality: string;
}

/** Bits por peso efectivos aproximados de los GGUF habituales. */
export const QUANTS: Record<string, QuantInfo> = {
  Q2_K: { bpw: 2.96, quality: 'Baja: pérdida notable' },
  Q3_K_M: { bpw: 3.91, quality: 'Aceptable: algo de pérdida' },
  Q4_0: { bpw: 4.54, quality: 'Buena (formato clásico)' },
  Q4_K_M: { bpw: 4.85, quality: 'Buena: el equilibrio recomendado' },
  Q5_K_M: { bpw: 5.69, quality: 'Muy buena' },
  Q6_K: { bpw: 6.56, quality: 'Casi sin pérdida' },
  Q8_0: { bpw: 8.5, quality: 'Prácticamente original' },
  F16: { bpw: 16, quality: 'Original (16 bits)' },
  MXFP4: { bpw: 4.6, quality: 'Nativa del modelo (sin pérdida extra)' },
};

/** Ancho de banda teórico de RAM en doble canal (GB/s). */
export const RAM_BANDWIDTH: Record<RamType, number> = {
  ddr4: 51,
  ddr5: 83,
  lpddr5x: 120,
  unknown: 51,
};

/**
 * Fracción del ancho de banda teórico que aprovecha llama.cpp, y coste fijo por token.
 * Calibrado con mediciones públicas (shared/catalog/benchmarks.json):
 * CUDA aprovecha mejor la memoria y lanza los kernels más rápido que Metal.
 * AMD e Intel no están validados aún: usan los valores conservadores.
 */
const GPU_EFF: Record<Gpu['vendor'], { bw: number; tokenOverheadS: number }> = {
  nvidia: { bw: 0.85, tokenOverheadS: 0.001 },
  apple: { bw: 0.75, tokenOverheadS: 0.002 },
  amd: { bw: 0.75, tokenOverheadS: 0.002 },
  intel: { bw: 0.75, tokenOverheadS: 0.002 },
};
const CPU_BW_EFF = 0.65;
/** Cada hilo de CPU satura aprox. esta cantidad de GB/s de memoria. */
const CPU_BW_PER_THREAD = 8;
const GPU_FLOPS_EFF = 0.5;
const CPU_TFLOPS_PER_THREAD = 0.05;
/** VRAM que se queda el sistema/escritorio/driver. */
const VRAM_RESERVE_GB = 0.6;
/** RAM que se reserva para el sistema operativo y apps. */
const RAM_RESERVE_GB = 3;
/** Buffers de cómputo de llama.cpp. */
const COMPUTE_OVERHEAD_GB = 0.5;
/** Fracción de memoria unificada que macOS deja usar a la GPU. */
const UNIFIED_GPU_FRACTION = 0.7;

export type Placement = 'gpu' | 'hybrid' | 'cpu' | 'nofit';
export type Rating = 'excelente' | 'fluida' | 'usable' | 'lenta' | 'muy-lenta' | 'no-cabe';

export interface Estimate {
  quant: string;
  context: number;
  weightsGB: number;
  kvCacheGB: number;
  overheadGB: number;
  totalGB: number;
  placement: Placement;
  /** Fracción del modelo cargada en GPU (0–1). */
  gpuFraction: number;
  gpuMemGB: number;
  ramMemGB: number;
  genTokS: number;
  prefillTokS: number;
  rating: Rating;
}

export function quantBpw(quant: string): number {
  const q = QUANTS[quant];
  if (!q) throw new Error(`Cuantización desconocida: ${quant}`);
  return q.bpw;
}

export function weightsGB(model: Model, quant: string): number {
  return (model.paramsB * quantBpw(quant)) / 8;
}

/** Caché KV en f16: 2 (K y V) · capas · cabezas KV · dim · contexto · 2 bytes. */
export function kvCacheGB(model: Model, context: number): number {
  const bytes = 2 * model.layers * model.kvHeads * model.headDim * context * 2;
  return (bytes * (model.kvCacheScale ?? 1)) / 1e9;
}

export function rate(genTokS: number, placement: Placement): Rating {
  if (placement === 'nofit') return 'no-cabe';
  if (genTokS >= 40) return 'excelente';
  if (genTokS >= 15) return 'fluida';
  if (genTokS >= 6) return 'usable';
  if (genTokS >= 2) return 'lenta';
  return 'muy-lenta';
}

export function effectiveVramGB(hw: HardwareProfile, gpu: Gpu | null): number {
  if (!gpu) return 0;
  if (gpu.unifiedMemory) return hw.ramGB * UNIFIED_GPU_FRACTION;
  return hw.vramGBOverride ?? gpu.vramGB;
}

export function estimate(
  model: Model,
  quant: string,
  context: number,
  hw: HardwareProfile,
  gpu: Gpu | null,
): Estimate {
  const w = weightsGB(model, quant);
  const kv = kvCacheGB(model, context);
  const overhead = COMPUTE_OVERHEAD_GB;
  const payload = w + kv;
  const total = payload + overhead;

  const vram = effectiveVramGB(hw, gpu);
  const unified = !!gpu?.unifiedMemory;
  const ramUsable = Math.max(0, hw.ramGB - RAM_RESERVE_GB - (unified ? vram : 0));

  let gpuFraction = 0;
  if (gpu && vram > 0) {
    const gpuUsable = vram - (unified ? 0 : VRAM_RESERVE_GB) - overhead;
    gpuFraction = Math.max(0, Math.min(1, gpuUsable / payload));
  }
  const gpuMem = gpuFraction > 0 ? gpuFraction * payload + overhead : 0;
  const ramMem = (1 - gpuFraction) * payload + (gpuFraction > 0 ? 0 : overhead);

  let placement: Placement;
  if (gpuFraction >= 1) placement = 'gpu';
  else if (ramMem > ramUsable) placement = 'nofit';
  else if (gpuFraction > 0) placement = 'hybrid';
  else placement = 'cpu';

  // Bytes leídos por token: pesos activos + caché KV media (contexto a medio llenar).
  const activeRatio = (model.activeParamsB ?? model.paramsB) / model.paramsB;
  const bytesPerToken = w * activeRatio + kv * 0.5;

  const cpuBw =
    Math.min(RAM_BANDWIDTH[hw.ramType], hw.cpuThreads * CPU_BW_PER_THREAD) * CPU_BW_EFF;
  const eff = gpu ? GPU_EFF[gpu.vendor] : null;
  const gpuBw = gpu && eff ? gpu.bandwidthGBs * eff.bw : 0;

  let tGen = 0;
  if (gpuFraction > 0 && eff) tGen += (gpuFraction * bytesPerToken) / gpuBw + eff.tokenOverheadS;
  if (gpuFraction < 1) tGen += ((1 - gpuFraction) * bytesPerToken) / cpuBw;
  const genTokS = placement === 'nofit' ? 0 : 1 / tGen;

  const activeParams = (model.activeParamsB ?? model.paramsB) * 1e9;
  const gpuPrefill = gpu ? (gpu.fp16Tflops * 1e12 * GPU_FLOPS_EFF) / (2 * activeParams) : 0;
  const cpuPrefill = (hw.cpuThreads * CPU_TFLOPS_PER_THREAD * 1e12) / (2 * activeParams);
  let tPre = 0;
  if (gpuFraction > 0) tPre += gpuFraction / gpuPrefill;
  if (gpuFraction < 1) tPre += (1 - gpuFraction) / cpuPrefill;
  const prefillTokS = placement === 'nofit' ? 0 : 1 / tPre;

  return {
    quant,
    context,
    weightsGB: w,
    kvCacheGB: kv,
    overheadGB: overhead,
    totalGB: total,
    placement,
    gpuFraction,
    gpuMemGB: gpuMem,
    ramMemGB: ramMem,
    genTokS,
    prefillTokS,
    rating: rate(genTokS, placement),
  };
}

const DEFAULT_CONTEXT = 8192;

export function defaultContext(model: Model): number {
  return Math.min(DEFAULT_CONTEXT, model.maxContext);
}

/**
 * Elige la mejor cuantización para el equipo: la de más calidad que quepa
 * entera en GPU; si ninguna cabe, la por defecto del modelo; si esa no cabe,
 * la más pequeña disponible.
 */
export function recommend(model: Model, hw: HardwareProfile, gpu: Gpu | null, context = defaultContext(model)): Estimate {
  const byQuality = [...model.quants].sort((a, b) => quantBpw(b) - quantBpw(a));
  const estimates = byQuality.map((q) => estimate(model, q, context, hw, gpu));
  const fullGpu = estimates.find((e) => e.placement === 'gpu' && quantBpw(e.quant) <= 8.5);
  if (fullGpu) return fullGpu;
  const def = estimates.find((e) => e.quant === model.defaultQuant);
  if (def && def.placement !== 'nofit') return def;
  const fitting = estimates.filter((e) => e.placement !== 'nofit');
  return fitting.at(-1) ?? estimates.at(-1)!;
}

export const RATING_LABEL: Record<Rating, string> = {
  excelente: 'Excelente',
  fluida: 'Fluida',
  usable: 'Usable',
  lenta: 'Lenta',
  'muy-lenta': 'Muy lenta',
  'no-cabe': 'No cabe',
};

export const PLACEMENT_LABEL: Record<Placement, string> = {
  gpu: 'Todo en GPU',
  hybrid: 'GPU + RAM',
  cpu: 'Solo CPU',
  nofit: 'Memoria insuficiente',
};
