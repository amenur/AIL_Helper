<script lang="ts">
  import { onDestroy } from 'svelte';
  import { $hardware as hardware } from '../lib/store.ts';
  import {
    estimate,
    recommend,
    effectiveVramGB,
    QUANTS,
    RATING_LABEL,
    PLACEMENT_LABEL,
    defaultContext,
  } from '../lib/estimate.ts';
  import { buildInstallPlan, guessOs, type OsFamily } from '../lib/install.ts';
  import AgentInstall from './AgentInstall.svelte';
  import { gb, tps, ctxLabel, RATING_CLASS } from '../lib/format.ts';
  import type { Gpu, Model } from '../lib/types.ts';

  let { model, gpus }: { model: Model; gpus: Gpu[] } = $props();

  const SAMPLE = `¡Claro! Te explico cómo funciona un modelo de lenguaje ejecutado en local.

Cuando escribes un mensaje, llama.cpp primero procesa todo el prompt de golpe (la fase de "prefill"), que depende sobre todo de la potencia de cálculo de tu GPU o CPU. Después genera la respuesta token a token: en cada paso tiene que leer de memoria todos los pesos activos del modelo, por eso la velocidad de generación depende casi por completo del ancho de banda de la memoria.

Si el modelo cabe entero en la VRAM de la GPU, la generación es rápida. Si una parte tiene que quedarse en la RAM del sistema, esa parte marca el ritmo y todo va más lento. Por eso elegir bien la cuantización (Q4_K_M, Q5_K_M, Q8_0…) es la decisión más importante: menos bits por peso significa un modelo más pequeño y más rápido, a cambio de algo de calidad.

Con esta configuración puedes hacerte una idea de cómo se sentirá el chat en tu equipo antes de descargar nada.`;
  const PROMPT_TOKENS = 350;
  const tokens = SAMPLE.match(/\s*[^\s]{1,4}|\s+/g) ?? [];

  const contexts = [2048, 4096, 8192, 16384, 32768, 65536, 131072].filter((c) => c <= model.maxContext);

  const hw = $derived($hardware);
  const gpu = $derived(hw?.gpuId ? (gpus.find((g) => g.id === hw.gpuId) ?? null) : null);

  let quant = $state<string | null>(null);
  let context = $state(defaultContext(model));
  let os = $state<OsFamily>(guessOs());

  // Al conocer el hardware, preselecciona la cuantización recomendada.
  $effect(() => {
    if (hw && quant === null) quant = recommend(model, hw, gpu, context).quant;
  });

  const est = $derived(hw && quant ? estimate(model, quant, context, hw, gpu) : null);
  const plan = $derived(est ? buildInstallPlan(model, est, gpu, os) : null);
  const vram = $derived(hw ? effectiveVramGB(hw, gpu) : 0);

  // ---- Simulación del chat ----
  type Phase = 'idle' | 'prefill' | 'gen' | 'done';
  let phase = $state<Phase>('idle');
  let shown = $state(0);
  let elapsed = $state(0);
  let raf = 0;

  function stop() {
    cancelAnimationFrame(raf);
  }

  function simulate() {
    if (!est || est.placement === 'nofit') return;
    stop();
    const prefillS = PROMPT_TOKENS / est.prefillTokS;
    const rate = est.genTokS;
    const start = performance.now();
    phase = 'prefill';
    shown = 0;
    const tick = (now: number) => {
      const t = (now - start) / 1000;
      elapsed = t;
      if (t < prefillS) {
        phase = 'prefill';
      } else {
        phase = 'gen';
        shown = Math.min(tokens.length, Math.floor((t - prefillS) * rate));
        if (shown >= tokens.length) {
          phase = 'done';
          return;
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
  }

  onDestroy(stop);

  let copied = $state<string | null>(null);
  async function copy(id: string, text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = id;
      setTimeout(() => (copied = null), 1500);
    } catch {
      /* portapapeles no disponible */
    }
  }

  const pct = (used: number, cap: number) => `${Math.min(100, cap > 0 ? (used / cap) * 100 : 0)}%`;
</script>

{#if !hw || !est || !quant}
  <div class="card text-sm text-slate-400">Analizando tu equipo…</div>
{:else}
  <div class="grid gap-6 lg:grid-cols-5">
    <!-- Configuración y métricas -->
    <section class="card lg:col-span-2">
      <h2 class="text-lg font-semibold text-slate-100">Configuración</h2>

      <div class="mt-4">
        <span class="label">Cuantización</span>
        <div class="mt-2 flex flex-wrap gap-2">
          {#each model.quants as q}
            {@const e = estimate(model, q, context, hw, gpu)}
            <button
              class="rounded-lg border px-3 py-1.5 text-left text-xs {quant === q ? 'border-sky-400 bg-sky-400/10' : 'border-slate-700 hover:border-slate-500'}"
              onclick={() => (quant = q)}
            >
              <span class="font-mono text-slate-100">{q}</span>
              <span class="block text-slate-500">{gb(e.weightsGB)}</span>
            </button>
          {/each}
        </div>
        <p class="mt-2 text-xs text-slate-500">{QUANTS[quant]?.quality}</p>
      </div>

      <label class="mt-5 block">
        <span class="label">Contexto: {ctxLabel(context)} tokens</span>
        <input
          type="range"
          class="mt-2 w-full accent-sky-400"
          min="0"
          max={contexts.length - 1}
          step="1"
          value={contexts.indexOf(context)}
          oninput={(e) => (context = contexts[Number(e.currentTarget.value)])}
        />
        <div class="flex justify-between text-[10px] text-slate-500">
          {#each contexts as c}<span>{ctxLabel(c)}</span>{/each}
        </div>
      </label>

      <div class="mt-6 space-y-3">
        {#if gpu}
          <div>
            <div class="flex justify-between text-xs"><span class="text-slate-400">GPU ({gpu.name})</span><span class="text-slate-300">{gb(est.gpuMemGB)} / {gb(vram)}</span></div>
            <div class="mt-1 h-2 rounded-full bg-slate-800"><div class="h-2 rounded-full bg-sky-400" style:width={pct(est.gpuMemGB, vram)}></div></div>
          </div>
        {/if}
        <div>
          <div class="flex justify-between text-xs"><span class="text-slate-400">RAM del sistema</span><span class="text-slate-300">{gb(est.ramMemGB)} / {hw.ramGB} GB</span></div>
          <div class="mt-1 h-2 rounded-full bg-slate-800">
            <div class="h-2 rounded-full {est.placement === 'nofit' ? 'bg-rose-500' : 'bg-amber-400'}" style:width={pct(est.ramMemGB, hw.ramGB)}></div>
          </div>
        </div>
        <p class="text-xs text-slate-500">
          Pesos {gb(est.weightsGB)} · caché KV {gb(est.kvCacheGB)} · buffers {gb(est.overheadGB)}
        </p>
      </div>

      <dl class="mt-6 grid grid-cols-2 gap-4 border-t border-slate-800 pt-5">
        <div>
          <dt class="label">Generación</dt>
          <dd class="mt-1 font-mono text-2xl text-slate-100">{tps(est.genTokS)} <span class="text-sm text-slate-500">tok/s</span></dd>
        </div>
        <div>
          <dt class="label">Lectura del prompt</dt>
          <dd class="mt-1 font-mono text-2xl text-slate-100">{tps(est.prefillTokS)} <span class="text-sm text-slate-500">tok/s</span></dd>
        </div>
        <div>
          <dt class="label">Ejecución</dt>
          <dd class="mt-1 text-sm text-slate-200">
            {PLACEMENT_LABEL[est.placement]}{#if est.placement === 'hybrid'} ({Math.round(est.gpuFraction * 100)} % en GPU){/if}
          </dd>
        </div>
        <div>
          <dt class="label">Experiencia</dt>
          <dd class="mt-1"><span class="rounded-full border px-2 py-0.5 text-xs {RATING_CLASS[est.rating]}">{RATING_LABEL[est.rating]}</span></dd>
        </div>
      </dl>
    </section>

    <!-- Simulación del chat -->
    <section class="card flex flex-col lg:col-span-3">
      <div class="flex items-center justify-between">
        <h2 class="text-lg font-semibold text-slate-100">Simulación del chat</h2>
        <button class="btn" onclick={simulate} disabled={est.placement === 'nofit'}>
          {phase === 'idle' ? 'Simular respuesta' : 'Repetir'}
        </button>
      </div>
      <div class="mt-4 flex-1 space-y-3 rounded-lg border border-slate-800 bg-slate-950 p-4 text-sm">
        <div class="ml-auto w-fit max-w-[85%] rounded-lg bg-sky-500/15 px-3 py-2 text-sky-100">
          ¿Puedes explicarme cómo funciona un LLM en local?
        </div>
        <div class="min-h-48 max-w-[95%] whitespace-pre-wrap rounded-lg bg-slate-800/50 px-3 py-2 text-slate-200">
          {#if est.placement === 'nofit'}
            <span class="text-rose-300">Este modelo no cabe en la memoria de tu equipo con esta configuración. Prueba una cuantización menor, menos contexto u otro modelo.</span>
          {:else if phase === 'idle'}
            <span class="text-slate-500">Pulsa «Simular respuesta» para ver la velocidad estimada en tiempo real.</span>
          {:else if phase === 'prefill'}
            <span class="animate-pulse text-slate-400">Procesando el prompt ({PROMPT_TOKENS} tokens)…</span>
          {:else}
            {tokens.slice(0, shown).join('')}{#if phase === 'gen'}<span class="animate-pulse text-sky-300">▍</span>{/if}
          {/if}
        </div>
      </div>
      <p class="mt-3 text-xs text-slate-500">
        {#if phase !== 'idle'}
          {elapsed.toFixed(1)} s · {shown} / {tokens.length} tokens ·
        {/if}
        Primer token tras ≈ {(PROMPT_TOKENS / est.prefillTokS).toFixed(1)} s
      </p>
    </section>
  </div>

  <!-- Plan de instalación -->
  {#if plan && est.placement !== 'nofit'}
    <section class="card mt-6">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 class="text-lg font-semibold text-slate-100">Instalación: llama.cpp + Open WebUI</h2>
          <p class="mt-1 text-sm text-slate-400">
            Backend <strong class="text-slate-200">{plan.backendLabel}</strong> · {model.name} {est.quant} · contexto {ctxLabel(context)}
          </p>
        </div>
        <div class="flex rounded-lg border border-slate-700 p-0.5 text-xs">
          {#each [['windows', 'Windows'], ['linux', 'Linux']] as [v, l]}
            <button class="rounded-md px-3 py-1 {os === v ? 'bg-slate-700 text-slate-100' : 'text-slate-400'}" onclick={() => (os = v as OsFamily)}>{l}</button>
          {/each}
        </div>
      </div>

      <div class="mt-5">
        <AgentInstall {model} quant={est.quant} {context} gpuLayers={plan.gpuLayers} />
      </div>

      <details class="group mt-6">
        <summary class="cursor-pointer text-sm font-semibold text-slate-300 hover:text-slate-100">
          Prefiero instalarlo a mano <span class="text-slate-500 group-open:hidden">(ver comandos)</span>
        </summary>
      <ol class="mt-3 space-y-5 text-sm">
        <li>
          <p class="text-slate-200">1. Descarga llama.cpp desde <a class="text-sky-300 hover:underline" href="https://github.com/ggml-org/llama.cpp/releases/latest" target="_blank" rel="noopener">las releases oficiales</a> y descomprímelo:</p>
          <code class="code mt-2">{plan.llamaAsset}</code>
          {#if plan.backendNote}<p class="mt-1 text-xs text-amber-200/80">{plan.backendNote}</p>{/if}
        </li>
        <li>
          <p class="text-slate-200">
            2. Arranca el servidor; descargará el modelo de <a class="text-sky-300 hover:underline" href={`https://huggingface.co/${model.hfRepo}`} target="_blank" rel="noopener">{model.hfRepo}</a> la primera vez:
          </p>
          <div class="relative mt-2">
            <code class="code pr-20">{plan.llamaCommand}</code>
            <button class="btn-ghost absolute right-2 top-2 py-0.5 text-xs" onclick={() => copy('llama', plan.llamaCommand)}>{copied === 'llama' ? '✓' : 'Copiar'}</button>
          </div>
          {#if plan.gpuLayers > 0 && plan.gpuLayers < 999}
            <p class="mt-1 text-xs text-slate-500">-ngl {plan.gpuLayers}: carga {plan.gpuLayers} de {model.layers} capas en la GPU y el resto en RAM.</p>
          {/if}
        </li>
        <li>
          <p class="text-slate-200">3. Instala <a class="text-sky-300 hover:underline" href="https://docs.astral.sh/uv/" target="_blank" rel="noopener">uv</a> (no hace falta tener Python):</p>
          <div class="relative mt-2">
            <code class="code pr-20">{plan.uvInstall}</code>
            <button class="btn-ghost absolute right-2 top-2 py-0.5 text-xs" onclick={() => copy('uv', plan.uvInstall)}>{copied === 'uv' ? '✓' : 'Copiar'}</button>
          </div>
        </li>
        <li>
          <p class="text-slate-200">4. En otra terminal, arranca Open WebUI conectado a llama.cpp (la primera vez descarga varios GB):</p>
          <div class="relative mt-2">
            <code class="code pr-20">{plan.openWebUiCommand}</code>
            <button class="btn-ghost absolute right-2 top-2 py-0.5 text-xs" onclick={() => copy('webui', plan.openWebUiCommand)}>{copied === 'webui' ? '✓' : 'Copiar'}</button>
          </div>
        </li>
        <li>
          <p class="text-slate-200">5. Abre <span class="font-mono text-sky-300">{plan.chatUrl}</span>, crea la cuenta de administrador local y empieza a chatear.</p>
        </li>
      </ol>
      </details>
    </section>
  {/if}
{/if}
