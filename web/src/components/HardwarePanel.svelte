<script lang="ts">
  import { onMount } from 'svelte';
  import { $hardware as hardware, setHardware } from '../lib/store.ts';
  import {
    AGENT_DOWNLOAD_URL,
    detectInBrowser,
    fetchAgentReport,
    isSoftwareRenderer,
    profileFromAgent,
    profileFromBrowser,
    type BrowserDetection,
  } from '../lib/hardware.ts';
  import { effectiveVramGB } from '../lib/estimate.ts';
  import type { Gpu, HardwareProfile, RamType } from '../lib/types.ts';

  let { gpus, compact = false }: { gpus: Gpu[]; compact?: boolean } = $props();

  let detecting = $state(false);
  let browserInfo = $state<BrowserDetection | null>(null);
  let editing = $state(!compact);

  const vendors: [Gpu['vendor'], string][] = [
    ['nvidia', 'NVIDIA'],
    ['amd', 'AMD'],
    ['intel', 'Intel'],
    ['apple', 'Apple Silicon'],
  ];
  const baseRamOptions = [8, 16, 24, 32, 48, 64, 96, 128, 192, 256];
  const ramTypes: [RamType, string][] = [
    ['unknown', 'No lo sé'],
    ['ddr4', 'DDR4'],
    ['ddr5', 'DDR5'],
    ['lpddr5x', 'LPDDR5X (portátil)'],
  ];

  const hw = $derived($hardware);
  // Incluye el valor real (p. ej. 31 GB que informa el agente) aunque no sea estándar.
  const ramOptions = $derived(
    hw && !baseRamOptions.includes(hw.ramGB) ? [...baseRamOptions, hw.ramGB].sort((a, b) => a - b) : baseRamOptions,
  );
  const gpu = $derived(hw?.gpuId ? (gpus.find((g) => g.id === hw.gpuId) ?? null) : null);

  async function detect() {
    detecting = true;
    try {
      const report = await fetchAgentReport();
      if (report) {
        setHardware(profileFromAgent(report, gpus));
        return;
      }
      browserInfo = await detectInBrowser();
      setHardware(profileFromBrowser(browserInfo, gpus));
    } finally {
      detecting = false;
    }
  }

  function update(patch: Partial<HardwareProfile>) {
    if (!hw) return;
    setHardware({ ...hw, ...patch, source: hw.source === 'agent' ? 'agent' : 'manual' });
  }

  function onGpuChange(id: string) {
    update({ gpuId: id || null, vramGBOverride: null });
  }

  onMount(async () => {
    if (!hw) return detect();
    // Aunque haya un perfil guardado, si el agente está en marcha manda él.
    const report = await fetchAgentReport();
    if (report) setHardware(profileFromAgent(report, gpus));
  });

  const sourceLabel = $derived(
    hw?.source === 'agent'
      ? { text: 'Datos reales (agente local)', cls: 'text-lime-300 border-lime-400/40' }
      : hw?.source === 'manual'
        ? { text: 'Ajustado a mano', cls: 'text-sky-300 border-sky-400/40' }
        : { text: 'Detección aproximada del navegador', cls: 'text-amber-300 border-amber-400/40' },
  );
</script>

<section class="card">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div class="flex items-center gap-3">
      <h2 class="text-lg font-semibold text-slate-100">Tu equipo</h2>
      {#if hw}
        <span class="rounded-full border px-2 py-0.5 text-xs {sourceLabel.cls}">{sourceLabel.text}</span>
      {/if}
    </div>
    <div class="flex gap-2">
      {#if compact}
        <button class="btn-ghost" onclick={() => (editing = !editing)}>{editing ? 'Ocultar' : 'Ajustar'}</button>
      {/if}
      <button class="btn-ghost" onclick={detect} disabled={detecting}>
        {detecting ? 'Detectando…' : 'Volver a detectar'}
      </button>
    </div>
  </div>

  {#if !hw}
    <p class="mt-4 text-sm text-slate-400">Analizando el hardware…</p>
  {:else}
    <dl class="mt-4 grid grid-cols-2 gap-4 sm:grid-cols-4">
      <div>
        <dt class="label">GPU</dt>
        <dd class="mt-1 text-sm font-medium text-slate-100">{gpu?.name ?? 'Sin GPU dedicada'}</dd>
      </div>
      <div>
        <dt class="label">{gpu?.unifiedMemory ? 'Memoria para GPU' : 'VRAM'}</dt>
        <dd class="mt-1 text-sm font-medium text-slate-100">
          {gpu ? `${Math.round(effectiveVramGB(hw, gpu))} GB` : '—'}
          {#if gpu}<span class="text-slate-500"> · {gpu.bandwidthGBs} GB/s</span>{/if}
        </dd>
      </div>
      <div>
        <dt class="label">RAM</dt>
        <dd class="mt-1 text-sm font-medium text-slate-100">{hw.ramGB} GB {hw.ramType !== 'unknown' ? hw.ramType.toUpperCase() : ''}</dd>
      </div>
      <div>
        <dt class="label">CPU</dt>
        <dd class="mt-1 text-sm font-medium text-slate-100">{hw.cpuThreads} hilos</dd>
      </div>
    </dl>

    {#if hw.agent}
      <p class="mt-4 text-xs text-slate-500">
        {hw.agent.cpuName} · {hw.agent.os}
        {#if hw.agent.ramSpeedMTs} · RAM a {hw.agent.ramSpeedMTs} MT/s{/if}
        {#if hw.agent.cudaVersion} · CUDA {hw.agent.cudaVersion}{/if}
        {#if hw.agent.driverVersion} · driver {hw.agent.driverVersion}{/if}
        {#if hw.agent.diskFreeGB != null} · {Math.round(hw.agent.diskFreeGB)} GB libres en disco{/if}
        · agente v{hw.agent.version}
      </p>
    {/if}

    {#if hw.source === 'browser'}
      <p class="mt-4 rounded-lg border border-amber-400/20 bg-amber-400/5 p-3 text-xs text-amber-200/90">
        El navegador no puede leer la VRAM ni la RAM exacta (como mucho informa de 8 GB).
        {#if hw.detectedGpuName && !gpu}
          {#if isSoftwareRenderer(hw.detectedGpuName)}
            Tu navegador tiene la aceleración gráfica desactivada, así que no puede identificar la GPU.
          {:else}
            Ha detectado <code class="text-amber-100">{hw.detectedGpuName}</code>, pero no está en el catálogo.
          {/if}
        {/if}
        Revisa los valores abajo, o
        <a class="underline hover:text-amber-100" href={AGENT_DOWNLOAD_URL} target="_blank" rel="noopener">instala el agente local</a>
        para obtener datos exactos.
      </p>
    {/if}

    {#if editing}
      <div class="mt-5 grid gap-4 border-t border-slate-800 pt-5 sm:grid-cols-2 lg:grid-cols-4">
        <label class="block">
          <span class="label">Tarjeta gráfica</span>
          <select class="input mt-1" value={hw.gpuId ?? ''} onchange={(e) => onGpuChange(e.currentTarget.value)}>
            <option value="">Sin GPU dedicada / integrada</option>
            {#each vendors as [vendor, label]}
              <optgroup {label}>
                {#each gpus.filter((g) => g.vendor === vendor) as g}
                  <option value={g.id}>{g.name}{g.vramGB ? ` (${g.vramGB} GB)` : ''}</option>
                {/each}
              </optgroup>
            {/each}
          </select>
        </label>
        {#if gpu && !gpu.unifiedMemory}
          <label class="block">
            <span class="label">VRAM (GB)</span>
            <input
              class="input mt-1"
              type="number"
              min="1"
              max="192"
              value={hw.vramGBOverride ?? gpu.vramGB}
              onchange={(e) => update({ vramGBOverride: Number(e.currentTarget.value) || null })}
            />
          </label>
        {/if}
        <label class="block">
          <span class="label">Memoria RAM</span>
          <select class="input mt-1" value={hw.ramGB} onchange={(e) => update({ ramGB: Number(e.currentTarget.value) })}>
            {#each ramOptions as r}<option value={r}>{r} GB</option>{/each}
          </select>
        </label>
        <label class="block">
          <span class="label">Tipo de RAM</span>
          <select class="input mt-1" value={hw.ramType} onchange={(e) => update({ ramType: e.currentTarget.value as RamType })}>
            {#each ramTypes as [v, l]}<option value={v}>{l}</option>{/each}
          </select>
        </label>
        <label class="block">
          <span class="label">Hilos de CPU</span>
          <input
            class="input mt-1"
            type="number"
            min="1"
            max="256"
            value={hw.cpuThreads}
            onchange={(e) => update({ cpuThreads: Math.max(1, Number(e.currentTarget.value) || 1) })}
          />
        </label>
      </div>
      {#if browserInfo?.webgpu && hw.source !== 'agent'}
        <p class="mt-3 text-xs text-slate-500">
          WebGPU: {browserInfo.webgpu.vendor} {browserInfo.webgpu.architecture} {browserInfo.webgpu.description}
        </p>
      {/if}
    {/if}
  {/if}
</section>
