<script lang="ts">
  import { onMount } from 'svelte';
  import * as agent from '../lib/agent.ts';
  import {
    AGENT_LINUX_URL,
    AGENT_WINDOWS_URL,
    detectInBrowser,
    fetchAgentReport,
    profileFromAgent,
    profileFromBrowser,
  } from '../lib/hardware.ts';
  import { buildInstallPlan } from '../lib/install.ts';
  import { recommendTier, speedInWords, tierOptions, verdict, VERDICT_TEXT, type TierId } from '../lib/plans.ts';
  import type { Gpu, HardwareProfile, Model } from '../lib/types.ts';

  let { models, gpus }: { models: Model[]; gpus: Gpu[] } = $props();

  type Step = 'detect' | 'download' | 'choose' | 'installing' | 'done';
  let step = $state<Step>('detect');
  let hw = $state<HardwareProfile | null>(null);
  let exact = $state(false); // datos del agente (exactos) o del navegador (aproximados)
  let os = $state<'windows' | 'linux' | 'mac' | 'other'>('windows');
  let agentSeen = $state(false);
  let chosen = $state<TierId | null>(null);
  let st = $state<agent.AgentStatus | null>(null);
  let error = $state<string | null>(null);
  let showDetails = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  const gpu = $derived(hw?.gpuId ? (gpus.find((g) => g.id === hw!.gpuId) ?? null) : null);
  const options = $derived(hw ? tierOptions(models, hw, gpu) : []);
  const rec = $derived(recommendTier(options));
  const v = $derived(options.length ? verdict(options) : null);
  const selected = $derived(options.find((o) => o.tier.id === (chosen ?? rec?.tier.id)) ?? null);

  // Pasos en lenguaje llano (los ids vienen del agente).
  const PLAIN: Record<string, string> = {
    llama: 'Preparando el motor',
    model: 'Descargando tu asistente',
    uv: 'Preparando herramientas',
    webui: 'Instalando la pantalla de chat',
    start: 'Poniéndolo en marcha',
  };

  function detectOs() {
    const ua = navigator.userAgent;
    if (/Windows/i.test(ua)) return 'windows';
    if (/Mac OS X|Macintosh/i.test(ua)) return 'mac';
    if (/Linux|X11/i.test(ua) && !/Android/i.test(ua)) return 'linux';
    return 'other';
  }

  async function useAgentHardware() {
    const report = await fetchAgentReport(2000);
    if (report) {
      hw = profileFromAgent(report, gpus);
      exact = true;
    }
  }

  /** Comprueba el agente y avanza solo cuando el usuario abre la app. */
  async function tick() {
    clearTimeout(timer);
    const h = await agent.health();
    agentSeen = !!h;
    if (h?.install && (await agent.isPaired())) {
      if (!exact) await useAgentHardware();
      try {
        st = await agent.status();
      } catch {
        st = null;
      }
      const job = st?.job;
      if (job?.phase === 'running') step = 'installing';
      else if (job?.phase === 'ready' && st?.services.webui) step = 'done';
      else if (step === 'download' || step === 'detect') step = 'choose';
      else if (step === 'installing' && job?.phase === 'error') error = job.error;
    }
    timer = setTimeout(tick, step === 'installing' ? 1000 : 2000);
  }

  async function install() {
    if (!selected?.model || !selected.est || !hw) return;
    error = null;
    const plan = buildInstallPlan(selected.model, selected.est, gpu, 'windows');
    try {
      await agent.install({
        modelId: selected.model.id,
        quant: selected.est.quant,
        context: selected.est.context,
        gpuLayers: Math.min(plan.gpuLayers, 999),
      });
      step = 'installing';
      tick();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(() => {
    os = detectOs();
    agent.setWizardActive(true);
    (async () => {
      hw = profileFromBrowser(await detectInBrowser(), gpus);
      await useAgentHardware();
      step = 'download';
      tick();
    })();
    return () => clearTimeout(timer);
  });

  const gbText = (n: number) => `${n < 10 ? n.toFixed(1).replace('.', ',') : Math.round(n)} GB`;
  const job = $derived(st?.job ?? null);
  const overall = $derived.by(() => {
    if (!job) return 0;
    const weight: Record<string, number> = { llama: 10, model: 50, uv: 5, webui: 25, start: 10 };
    let done = 0;
    for (const s of job.steps) {
      const w = weight[s.id] ?? 0;
      if (s.state === 'done' || s.state === 'skipped') done += w;
      else if (s.state === 'running') done += s.totalBytes ? (w * s.doneBytes) / s.totalBytes : w * 0.3;
    }
    return Math.min(100, Math.round(done));
  });
  // Si la pantalla de chat aún no está instalada, la primera vez se descarga además.
  const firstTime = $derived(!st?.installed?.webui);
  const stepIndex = $derived(['detect', 'download', 'choose', 'installing', 'done'].indexOf(step));
</script>

<ol class="space-y-4">
  <!-- 1. Tu ordenador -->
  <li class="card">
    <div class="flex items-start gap-4">
      <span class="grid h-9 w-9 shrink-0 place-items-center rounded-full {stepIndex > 0 && v !== 'no' ? 'bg-lime-400' : 'bg-sky-500'} font-semibold text-slate-950">
        {stepIndex > 0 && v !== 'no' ? '✓' : '1'}
      </span>
      <div class="min-w-0 flex-1">
        <h2 class="text-lg font-semibold text-slate-50">Tu ordenador</h2>
        {#if !hw || !v}
          <p class="mt-1 animate-pulse text-slate-400">Comprobando tu ordenador…</p>
        {:else if os === 'mac'}
          <p class="mt-1 text-amber-200">De momento la app solo funciona en Windows. La versión para Mac llegará pronto.</p>
        {:else}
          <p class="mt-1 text-lg {v === 'no' ? 'text-rose-300' : v === 'tight' ? 'text-amber-200' : 'text-lime-300'}">
            {v === 'no' ? '✕' : v === 'tight' ? '!' : '✓'} {VERDICT_TEXT[v].title}
          </p>
          <p class="text-slate-400">{VERDICT_TEXT[v].text}</p>
          {#if !exact}
            <p class="mt-2 text-sm text-slate-500">
              Es una primera estimación desde el navegador; la app lo comprobará con exactitud al instalar.
            </p>
          {/if}
        {/if}
      </div>
    </div>
  </li>

  <!-- 2. Descarga la app -->
  <li class="card {stepIndex < 1 ? 'opacity-50' : ''}">
    <div class="flex items-start gap-4">
      <span class="grid h-9 w-9 shrink-0 place-items-center rounded-full {stepIndex > 1 ? 'bg-lime-400' : 'bg-sky-500'} font-semibold text-slate-950">
        {stepIndex > 1 ? '✓' : '2'}
      </span>
      <div class="min-w-0 flex-1">
        <h2 class="text-lg font-semibold text-slate-50">Descarga y abre la app</h2>
        {#if stepIndex > 1}
          <p class="mt-1 text-slate-400">App abierta y conectada con esta página.</p>
        {:else if step === 'download' && os !== 'mac'}
          <p class="mt-1 text-slate-400">
            Es un programa pequeño (pocos MB). Se encarga de descargar e instalar todo lo demás por ti.
          </p>
          <div class="mt-4 flex flex-wrap items-center gap-3">
            {#if os === 'linux'}
              <a class="btn btn-lg" href={AGENT_LINUX_URL}>Descargar para Linux</a>
              <a class="text-sm text-slate-400 underline hover:text-slate-200" href={AGENT_WINDOWS_URL}>¿Usas Windows?</a>
            {:else}
              <a class="btn btn-lg" href={AGENT_WINDOWS_URL}>Descargar para Windows</a>
            {/if}
          </div>
          <div class="mt-5 rounded-lg border border-white/10 bg-white/[0.03] p-4 text-sm text-slate-300">
            <p class="font-medium text-slate-100">Después:</p>
            <ol class="mt-2 list-decimal space-y-1.5 pl-5">
              <li>Abre el archivo que acabas de descargar.</li>
              <li>
                Si Windows muestra <em>«Windows protegió su PC»</em>, pulsa <strong>«Más información»</strong> y luego
                <strong>«Ejecutar de todas formas»</strong>. Sale porque la app es nueva; es segura y su código es público.
              </li>
              <li>Se abrirá una pestaña nueva: ya puedes volver aquí.</li>
            </ol>
          </div>
          <p class="mt-4 flex items-center gap-2 text-sm text-sky-200" aria-live="polite">
            <span class="h-2 w-2 animate-pulse rounded-full bg-sky-400"></span>
            {agentSeen ? 'App encontrada, conectando…' : 'Esperando a que abras la app…'}
          </p>
        {/if}
      </div>
    </div>
  </li>

  <!-- 3. Elige tu asistente -->
  <li class="card {stepIndex < 2 ? 'opacity-50' : ''}">
    <div class="flex items-start gap-4">
      <span class="grid h-9 w-9 shrink-0 place-items-center rounded-full {stepIndex > 2 ? 'bg-lime-400' : 'bg-sky-500'} font-semibold text-slate-950">
        {stepIndex > 2 ? '✓' : '3'}
      </span>
      <div class="min-w-0 flex-1">
        <h2 class="text-lg font-semibold text-slate-50">Elige tu asistente</h2>
        {#if step === 'choose'}
          <p class="mt-1 text-slate-400">Te recomendamos uno según tu ordenador. Podrás cambiarlo cuando quieras.</p>
          <div class="mt-4 grid gap-3 sm:grid-cols-3" role="radiogroup" aria-label="Tipo de asistente">
            {#each options as o (o.tier.id)}
              {@const isSel = selected?.tier.id === o.tier.id}
              <button
                role="radio"
                aria-checked={isSel}
                disabled={!o.available}
                onclick={() => (chosen = o.tier.id)}
                class="relative rounded-xl border p-4 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-40
                  {isSel ? 'border-sky-400 bg-sky-400/10' : 'border-white/10 hover:border-white/30'}"
              >
                {#if rec?.tier.id === o.tier.id}
                  <span class="absolute -top-2.5 right-3 rounded-full bg-lime-400 px-2 py-0.5 text-[11px] font-semibold text-slate-950">Recomendado</span>
                {/if}
                <span class="block font-semibold text-slate-50">{o.tier.name}</span>
                <span class="mt-1 block text-sm text-slate-400">{o.tier.tagline}</span>
                {#if o.available && o.est}
                  <span class="mt-3 block text-xs text-slate-300">Responde {speedInWords(o.est)}</span>
                  <span class="block text-xs text-slate-500">Descarga de {gbText(o.est.weightsGB)}{firstTime ? ' + hasta 7 GB la primera vez' : ''}</span>
                {:else}
                  <span class="mt-3 block text-xs text-slate-500">Tu ordenador no llega para este</span>
                {/if}
              </button>
            {/each}
          </div>
          <button class="btn btn-lg mt-5" onclick={install} disabled={!selected?.available}>Instalar mi asistente</button>
          <p class="mt-2 text-xs text-slate-500">
            Todo se guarda en tu carpeta de usuario. La primera vez descarga varios GB; después funciona sin internet.
          </p>
        {:else if stepIndex > 2 && job?.modelId}
          <p class="mt-1 text-slate-400">
            {chosen ? `Asistente ${selected?.tier.name}` : (options.find((o) => o.model?.id === job.modelId)?.tier.name ?? 'Asistente instalado')}
          </p>
        {/if}
      </div>
    </div>
  </li>

  <!-- 4. Instalación -->
  <li class="card {stepIndex < 3 ? 'opacity-50' : ''}">
    <div class="flex items-start gap-4">
      <span class="grid h-9 w-9 shrink-0 place-items-center rounded-full {step === 'done' ? 'bg-lime-400' : 'bg-sky-500'} font-semibold text-slate-950">
        {step === 'done' ? '✓' : '4'}
      </span>
      <div class="min-w-0 flex-1" aria-live="polite">
        <h2 class="text-lg font-semibold text-slate-50">{step === 'done' ? '¡Tu asistente está listo!' : 'Instalación'}</h2>
        {#if step === 'installing' && job}
          {@const current = job.steps.find((s) => s.state === 'running' || s.state === 'error')}
          <div class="mt-3 h-3 rounded-full bg-slate-800">
            <div class="h-3 rounded-full bg-gradient-to-r from-sky-400 to-lime-300 transition-[width] duration-500" style:width="{overall}%"></div>
          </div>
          <p class="mt-2 text-slate-300">
            {#if job.phase === 'error'}
              Algo ha fallado.
            {:else if current}
              {PLAIN[current.id] ?? current.label}…
              {#if current.totalBytes}
                <span class="text-slate-500">({gbText(current.doneBytes / 1e9)} de {gbText(current.totalBytes / 1e9)})</span>
              {/if}
            {:else}
              Preparando…
            {/if}
          </p>
          <p class="text-sm text-slate-500">Puedes seguir usando el ordenador. Si cierras esta página, la instalación continúa.</p>
          {#if job.phase === 'error'}
            <div class="mt-4 rounded-lg border border-rose-400/30 bg-rose-400/5 p-4">
              <p class="text-rose-200">No se ha podido completar la instalación.</p>
              <button class="mt-2 text-sm text-rose-200 underline" onclick={() => (showDetails = !showDetails)}>
                {showDetails ? 'Ocultar detalles' : 'Ver detalles (para soporte)'}
              </button>
              {#if showDetails}
                <pre class="mt-2 overflow-x-auto whitespace-pre-wrap text-xs text-rose-100/80">{job.error}</pre>
              {/if}
              <button class="btn mt-3" onclick={() => (step = 'choose')}>Volver a intentarlo</button>
            </div>
          {/if}
        {:else if step === 'done'}
          <p class="mt-1 text-slate-400">Se abre en tu navegador, como cualquier página web, pero funciona dentro de tu ordenador.</p>
          <a class="btn btn-lg mt-4" href={job?.chatUrl ?? 'http://localhost:3000'} target="_blank" rel="noopener">Abrir mi asistente →</a>
          <ul class="mt-5 space-y-1.5 text-sm text-slate-400">
            <li>💡 Guarda <strong class="text-slate-200">localhost:3000</strong> en favoritos para volver cuando quieras.</li>
            <li>💡 La app tiene que estar abierta para que el asistente funcione.</li>
          </ul>
        {/if}
        {#if error}
          <p class="mt-3 text-sm text-rose-300">{error}</p>
        {/if}
      </div>
    </div>
  </li>
</ol>
