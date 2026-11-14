import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { estimate, kvCacheGB, recommend, weightsGB, QUANTS } from './estimate.ts';
import { matchGpu } from './hardware.ts';
import type { Gpu, HardwareProfile, Model } from './types.ts';

const models: Model[] = JSON.parse(readFileSync(new URL('../../../shared/catalog/models.json', import.meta.url), 'utf8'));
const gpus: Gpu[] = JSON.parse(readFileSync(new URL('../../../shared/catalog/gpus.json', import.meta.url), 'utf8'));
const model = (id: string) => models.find((m) => m.id === id)!;
const gpu = (id: string) => gpus.find((g) => g.id === id)!;

const hw = (over: Partial<HardwareProfile> = {}): HardwareProfile => ({
  source: 'manual',
  cpuThreads: 16,
  ramGB: 32,
  ramType: 'ddr5',
  gpuId: null,
  ...over,
});

test('catálogo: cuantizaciones conocidas y ids únicos', () => {
  for (const m of models) {
    for (const q of m.quants) assert.ok(QUANTS[q], `${m.id}: ${q}`);
    assert.ok(m.quants.includes(m.defaultQuant), m.id);
  }
  assert.equal(new Set(models.map((m) => m.id)).size, models.length);
  assert.equal(new Set(gpus.map((g) => g.id)).size, gpus.length);
});

test('tamaños: Llama 3.1 8B Q4_K_M ≈ 4,9 GB y KV de 8k ≈ 1 GB', () => {
  const m = model('llama-3.1-8b');
  assert.ok(Math.abs(weightsGB(m, 'Q4_K_M') - 4.87) < 0.1);
  assert.ok(Math.abs(kvCacheGB(m, 8192) - 1.07) < 0.05);
});

test('RTX 4090 + 8B Q4: todo en GPU, ~100–150 tok/s', () => {
  const e = estimate(model('llama-3.1-8b'), 'Q4_K_M', 8192, hw(), gpu('rtx-4090'));
  assert.equal(e.placement, 'gpu');
  assert.ok(e.genTokS > 90 && e.genTokS < 160, String(e.genTokS));
});

test('solo CPU DDR5 + 8B Q4: ~8–14 tok/s', () => {
  const e = estimate(model('llama-3.1-8b'), 'Q4_K_M', 8192, hw(), null);
  assert.equal(e.placement, 'cpu');
  assert.ok(e.genTokS > 8 && e.genTokS < 14, String(e.genTokS));
});

test('70B en GPU de 8 GB y 16 GB de RAM: no cabe', () => {
  const e = estimate(model('llama-3.3-70b'), 'Q4_K_M', 8192, hw({ ramGB: 16 }), gpu('rtx-4060'));
  assert.equal(e.placement, 'nofit');
  assert.equal(e.rating, 'no-cabe');
});

test('descarga parcial es más lenta que todo en GPU y más rápida que solo CPU', () => {
  const m = model('qwen3-32b');
  const h = hw({ ramGB: 64 });
  const full = estimate(m, 'Q4_K_M', 8192, h, gpu('rtx-5090'));
  const hybrid = estimate(m, 'Q4_K_M', 8192, h, gpu('rtx-4070'));
  const cpu = estimate(m, 'Q4_K_M', 8192, h, null);
  assert.equal(full.placement, 'gpu');
  assert.equal(hybrid.placement, 'hybrid');
  assert.ok(full.genTokS > hybrid.genTokS && hybrid.genTokS > cpu.genTokS);
});

test('MoE: Qwen3 30B-A3B en CPU va mucho más rápido que un denso de 32B', () => {
  const moe = estimate(model('qwen3-30b-a3b'), 'Q4_K_M', 8192, hw({ ramGB: 64 }), null);
  const dense = estimate(model('qwen3-32b'), 'Q4_K_M', 8192, hw({ ramGB: 64 }), null);
  assert.ok(moe.genTokS > dense.genTokS * 4);
});

test('Apple: memoria unificada usa la RAM como VRAM', () => {
  const e = estimate(model('qwen3-14b'), 'Q4_K_M', 8192, hw({ ramGB: 32 }), gpu('apple-m3-max'));
  assert.equal(e.placement, 'gpu');
});

test('recommend elige la mejor calidad que cabe en GPU', () => {
  const r = recommend(model('llama-3.1-8b'), hw(), gpu('rtx-4090'));
  assert.equal(r.quant, 'Q8_0');
  const small = recommend(model('llama-3.1-8b'), hw(), gpu('rtx-4060'));
  assert.equal(small.placement, 'gpu');
  assert.ok(['Q4_K_M', 'Q5_K_M', 'Q6_K'].includes(small.quant));
});

test('matchGpu reconoce renderers de ANGLE y prefiere la coincidencia más larga', () => {
  const r = 'ANGLE (NVIDIA, NVIDIA GeForce RTX 4070 Ti SUPER (0x00002705) Direct3D11 vs_5_0 ps_5_0, D3D11)';
  assert.equal(matchGpu(r, gpus)?.id, 'rtx-4070-ti-super');
  assert.equal(matchGpu('ANGLE (NVIDIA, NVIDIA GeForce RTX 4070 (0x00002786) Direct3D11)', gpus)?.id, 'rtx-4070');
  assert.equal(matchGpu('NVIDIA GeForce RTX 4060 Laptop GPU/PCIe/SSE2', gpus)?.id, 'rtx-4060-laptop-gpu');
  assert.equal(matchGpu('AMD Radeon RX 7900 XTX (radeonsi, navi31, LLVM 17.0.6)', gpus)?.id, 'rx-7900-xtx');
  assert.equal(matchGpu('ANGLE (Apple, ANGLE Metal Renderer: Apple M2 Pro, Unspecified Version)', gpus)?.id, 'apple-m2-pro');
  assert.equal(matchGpu('Mesa Intel(R) UHD Graphics 620', gpus), null);
});
