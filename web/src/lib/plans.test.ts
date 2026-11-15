import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { recommendTier, speedInWords, tierOptions, verdict } from './plans.ts';
import type { Gpu, HardwareProfile, Model } from './types.ts';

const models: Model[] = JSON.parse(readFileSync(new URL('../../../shared/catalog/models.json', import.meta.url), 'utf8'));
const gpus: Gpu[] = JSON.parse(readFileSync(new URL('../../../shared/catalog/gpus.json', import.meta.url), 'utf8'));
const gpu = (id: string) => gpus.find((g) => g.id === id)!;
const hw = (over: Partial<HardwareProfile> = {}): HardwareProfile => ({
  source: 'manual', cpuThreads: 8, ramGB: 16, ramType: 'ddr4', gpuId: null, ...over,
});

test('opciones: cada una apunta a un modelo del catálogo', () => {
  for (const o of tierOptions(models, hw({ ramGB: 32, gpuId: 'rtx-4070' }), gpu('rtx-4070'))) {
    assert.ok(o.available, o.tier.id);
    assert.ok(models.includes(o.model!));
  }
});

test('ordenador potente: recomienda "Más inteligente" o "Equilibrado"', () => {
  const opts = tierOptions(models, hw({ ramGB: 64, cpuThreads: 24, gpuId: 'rtx-4090' }), gpu('rtx-4090'));
  assert.equal(verdict(opts), 'great');
  assert.notEqual(recommendTier(opts)!.tier.id, 'rapido');
});

test('portátil modesto sin GPU: recomienda "Rápido"', () => {
  const opts = tierOptions(models, hw({ ramGB: 8, cpuThreads: 4 }), null);
  assert.equal(recommendTier(opts)!.tier.id, 'rapido');
  assert.ok(['ok', 'tight'].includes(verdict(opts)));
});

test('equipo sin memoria: no llega', () => {
  assert.equal(verdict(tierOptions(models, hw({ ramGB: 4, cpuThreads: 2 }), null)), 'no');
});

test('dos opciones nunca son el mismo modelo', () => {
  for (const h of [hw({ ramGB: 16, cpuThreads: 4 }), hw({ ramGB: 32, gpuId: 'rtx-4060' }), hw({ ramGB: 64, gpuId: 'rtx-4090' })]) {
    const ids = tierOptions(models, h, h.gpuId ? gpu(h.gpuId) : null).filter((o) => o.available).map((o) => o.model!.id);
    assert.equal(new Set(ids).size, ids.length, ids.join(','));
  }
});

test('la velocidad se explica en palabras', () => {
  const [rapido] = tierOptions(models, hw({ ramGB: 32, gpuId: 'rtx-4090' }), gpu('rtx-4090'));
  assert.equal(speedInWords(rapido.est!), 'al momento');
});
