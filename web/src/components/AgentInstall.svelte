<script lang="ts">
  import { onMount } from 'svelte';
  import * as agent from '../lib/agent.ts';
  import { AGENT_DOWNLOAD_URL } from '../lib/hardware.ts';
  import type { Model } from '../lib/types.ts';

  let {
    model,
    quant,
    context,
    gpuLayers,
  }: { model: Model; quant: string; context: number; gpuLayers: number } = $props();

  type Conn = 'checking' | 'no-agent' | 'old-agent' | 'unpaired' | 'paired';
  let conn = $state<Conn>('checking');
  let st = $state<agent.AgentStatus | null>(null);
  let busy = $state(false);
  let message = $state<string | null>(null);
  let pasted = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;

  const job = $derived(st?.job ?? null);
  const running = $derived(job?.phase === 'running');
  const thisModel = $derived(job?.modelId === model.id && job?.quant === quant);
  const chatReady = $derived(!!st?.services.webui && !!st?.services.llama && job?.phase === 'ready');

  async function connect() {
    const h = await agent.health();
    if (!h) return (conn = 'no-agent');
    if (!h.install) return (conn = 'old-agent');
    if (!(await agent.isPaired())) return (conn = 'unpaired');
    conn = 'paired';
    await poll();
  }

  async function poll() {
    clearTimeout(timer);
    try {
      st = await agent.status();
    } catch (e) {
      if (e instanceof agent.AgentError && e.status === 401) conn = 'unpaired';
      else if (e instanceof agent.AgentError && e.status === 0) conn = 'no-agent';
      return;
    }
    // Mientras instala, se consulta cada segundo; después, cada 5 s para ver si los servicios siguen vivos.
    timer = setTimeout(poll, st.job.phase === 'running' ? 1000 : 5000);
  }

  async function start() {
    busy = true;
    message = null;
    try {
      await agent.install({ modelId: model.id, quant, context, gpuLayers: Math.min(gpuLayers, 999) });
      await poll();
    } catch (e) {
      message = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function stopServices() {
    busy = true;
    try {
      await agent.stop();
      await poll();
    } finally {
      busy = false;
    }
  }

  async function pairWithPasted() {
    const t = pasted.trim().replace(/^.*#token=/, '');
    if (!agent.isValidToken(t)) {
      message = 'Ese código no es válido: copia el enlace completo que muestra el agente.';
      return;
    }
    agent.setToken(t);
    message = null;
    await connect();
    if (conn !== 'paired') message = 'El código no coincide con el agente en marcha.';
  }

  onMount(() => {
    connect();
    return () => clearTimeout(timer);
  });

  const fmtBytes = (n: number) =>
    n >= 1e9 ? `${(n / 1e9).toFixed(2)} GB` : n >= 1e6 ? `${Math.round(n / 1e6)} MB` : `${Math.round(n / 1e3)} KB`;
  const ICON: Record<agent.StepState, string> = { pending: '○', running: '◌', done: '✓', skipped: '✓', error: '✕' };
  const ICON_CLASS: Record<agent.StepState, string> = {
    pending: 'text-slate-600',
    running: 'animate-spin text-sky-300',
    done: 'text-lime-400',
    skipped: 'text-slate-400',
    error: 'text-rose-400',
  };
</script>

<div class="rounded-lg border border-sky-400/30 bg-sky-400/5 p-4" aria-live="polite">
  {#if conn === 'checking'}
    <p class="text-sm text-sky-200/80">Buscando el agente local…</p>
  {:else if conn === 'no-agent'}
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <p class="font-medium text-sky-100">Instalación con un clic</p>
        <p class="text-xs text-sky-200/70">
          Descarga y abre el agente: instalará llama.cpp, el modelo y Open WebUI, y abrirá el chat. Luego vuelve aquí.
        </p>
      </div>
      <div class="flex gap-2">
        <button class="btn-ghost" onclick={connect}>Ya lo tengo abierto</button>
        <a class="btn" href={AGENT_DOWNLOAD_URL} target="_blank" rel="noopener">Descargar agente</a>
      </div>
    </div>
  {:else if conn === 'old-agent'}
    <p class="font-medium text-sky-100">Actualiza el agente</p>
    <p class="mt-1 text-xs text-sky-200/70">
      Tu versión del agente solo detecta el hardware. Descarga la última para instalar con un clic.
      <a class="underline" href={AGENT_DOWNLOAD_URL} target="_blank" rel="noopener">Descargar</a>
    </p>
  {:else if conn === 'unpaired'}
    <p class="font-medium text-sky-100">Vincula el agente con esta web</p>
    <p class="mt-1 text-xs text-sky-200/70">
      Al arrancar, el agente abre esta web automáticamente. Si no se abrió, copia aquí el enlace que muestra su ventana.
    </p>
    <form
      class="mt-3 flex gap-2"
      onsubmit={(e) => {
        e.preventDefault();
        pairWithPasted();
      }}
    >
      <input class="input font-mono text-xs" placeholder="https://…/vincular#token=…" bind:value={pasted} aria-label="Enlace de vinculación" />
      <button class="btn shrink-0" type="submit">Vincular</button>
    </form>
  {:else}
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <p class="font-medium text-sky-100">
          Agente conectado <span class="text-lime-300">✓</span>
        </p>
        <p class="text-xs text-sky-200/70">
          {#if chatReady && !thisModel}
            Ahora mismo sirve otro modelo ({job?.modelId} {job?.quant}). Instalar este lo sustituye.
          {:else}
            Instalará lo necesario en tu carpeta de usuario (~/.ail-helper) y arrancará el chat.
          {/if}
        </p>
      </div>
      <div class="flex gap-2">
        {#if chatReady}
          <button class="btn-ghost" onclick={stopServices} disabled={busy}>Detener</button>
        {/if}
        {#if chatReady && thisModel}
          <a class="btn" href={job?.chatUrl ?? 'http://localhost:3000'} target="_blank" rel="noopener">Abrir el chat →</a>
        {:else}
          <button class="btn" onclick={start} disabled={busy || running}>
            {running ? 'Instalando…' : 'Instalar y abrir el chat'}
          </button>
        {/if}
      </div>
    </div>

    {#if job && job.steps.length > 0 && (running || job.phase === 'error' || (thisModel && job.phase === 'ready'))}
      <ol class="mt-4 space-y-3 border-t border-sky-400/15 pt-4">
        {#each job.steps as step (step.id)}
          <li class="text-sm">
            <div class="flex items-center gap-2">
              <span class="inline-block w-4 text-center {ICON_CLASS[step.state]}">{ICON[step.state]}</span>
              <span class={step.state === 'pending' ? 'text-slate-500' : 'text-slate-200'}>{step.label}</span>
              {#if step.state === 'running' && step.totalBytes}
                <span class="ml-auto font-mono text-xs text-slate-400">
                  {fmtBytes(step.doneBytes)} / {fmtBytes(step.totalBytes)}
                </span>
              {/if}
            </div>
            {#if step.detail}
              <p class="ml-6 truncate text-xs text-slate-400">{step.detail}</p>
            {/if}
            {#if step.state === 'running' && step.totalBytes}
              <div class="mt-1.5 ml-6 h-1.5 rounded-full bg-slate-800">
                <div
                  class="h-1.5 rounded-full bg-sky-400 transition-[width]"
                  style:width="{Math.min(100, (step.doneBytes / step.totalBytes) * 100)}%"
                ></div>
              </div>
            {/if}
          </li>
        {/each}
      </ol>
    {/if}

    {#if job?.phase === 'error' && job.error}
      <p class="mt-3 whitespace-pre-line rounded-lg border border-rose-400/30 bg-rose-400/5 p-3 text-xs text-rose-200">{job.error}</p>
    {/if}
  {/if}

  {#if message}
    <p class="mt-3 text-xs text-rose-300">{message}</p>
  {/if}
</div>
