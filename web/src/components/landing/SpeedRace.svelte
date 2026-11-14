<script lang="ts">
  import { onMount } from 'svelte';
  import { estimate, RATING_LABEL, PLACEMENT_LABEL } from '../../lib/estimate.ts';
  import { DEMO_ANSWER, DEMO_MACHINES, DEMO_QUESTION, tokenize } from '../../lib/demo.ts';
  import { tps, wps, readingMultiple, RATING_CLASS } from '../../lib/format.ts';
  import type { Gpu, Model } from '../../lib/types.ts';

  let { models, gpus }: { models: Model[]; gpus: Gpu[] } = $props();

  const CHOICES = ['qwen3-8b', 'qwen3-14b', 'qwen3-30b-a3b'];
  // Plantilla de chat + pregunta: unos 60 tokens de prompt.
  const PROMPT_TOKENS = 60;
  // La carrera se corta aquí: el equipo lento no tiene por qué terminar.
  const MAX_RACE_S = 45;
  const tokens = tokenize(DEMO_ANSWER);

  const choices = $derived(CHOICES.map((id) => models.find((m) => m.id === id)).filter((m): m is Model => !!m));
  let modelId = $state(CHOICES[0]);
  const model = $derived(choices.find((m) => m.id === modelId) ?? choices[0]);

  const lanes = $derived(
    DEMO_MACHINES.map((m) => {
      const gpu = m.gpuId ? (gpus.find((g) => g.id === m.gpuId) ?? null) : null;
      return { machine: m, est: estimate(model, 'Q4_K_M', 8192, m.hw, gpu) };
    }),
  );

  let elapsed = $state(0);
  let running = $state(false);
  let raf = 0;
  let start = 0;
  let root: HTMLElement;

  function shownFor(genTokS: number, prefillTokS: number): number {
    if (genTokS <= 0) return 0;
    const t = elapsed - PROMPT_TOKENS / prefillTokS;
    return t <= 0 ? 0 : Math.min(tokens.length, Math.floor(t * genTokS));
  }

  function run() {
    cancelAnimationFrame(raf);
    start = performance.now();
    elapsed = 0;
    running = true;
    const tick = (now: number) => {
      elapsed = (now - start) / 1000;
      const done = lanes.every((l) => l.est.genTokS <= 0 || shownFor(l.est.genTokS, l.est.prefillTokS) >= tokens.length);
      if (done || elapsed > MAX_RACE_S) {
        running = false;
        return;
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
  }

  function pick(id: string) {
    modelId = id;
    run();
  }

  onMount(() => {
    const io = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          run();
          io.disconnect();
        }
      },
      { threshold: 0.35 },
    );
    io.observe(root);
    return () => {
      io.disconnect();
      cancelAnimationFrame(raf);
    };
  });
</script>

<div bind:this={root}>
  <div class="flex flex-wrap items-center justify-center gap-2">
    {#each choices as m}
      <button
        class="rounded-full border px-4 py-1.5 text-sm transition-colors {m.id === modelId
          ? 'border-sky-400 bg-sky-400/10 text-sky-100'
          : 'border-white/10 text-slate-400 hover:border-white/25 hover:text-slate-200'}"
        onclick={() => pick(m.id)}>{m.name}</button
      >
    {/each}
    <button class="ml-1 rounded-full border border-white/10 px-4 py-1.5 text-sm text-slate-400 hover:text-slate-200" onclick={run}>
      ↻ Repetir
    </button>
  </div>

  <div class="mx-auto mt-8 w-fit max-w-full rounded-2xl rounded-br-sm bg-sky-500/15 px-4 py-2.5 text-sm text-sky-50">
    <span class="mr-2 text-xs uppercase tracking-wider text-sky-300/80">Pregunta</span>{DEMO_QUESTION}
  </div>

  <div class="mt-6 grid gap-3 md:grid-cols-3 md:gap-4">
    {#each lanes as { machine, est } (machine.id)}
      {@const shown = shownFor(est.genTokS, est.prefillTokS)}
      {@const pct = (shown / tokens.length) * 100}
      <article class="glass flex flex-col overflow-hidden">
        <header class="border-b border-white/5 p-4">
          <div class="flex items-start justify-between gap-2">
            <div>
              <h3 class="font-semibold text-slate-100">{machine.label}</h3>
              <p class="text-xs text-slate-400">{machine.detail}</p>
            </div>
            <span class="rounded-full border px-2 py-0.5 text-xs {RATING_CLASS[est.rating]}">{RATING_LABEL[est.rating]}</span>
          </div>
          <div class="mt-3 flex items-baseline gap-2">
            <span class="font-mono text-3xl font-medium text-slate-50">{wps(est.genTokS)}</span>
            <span class="text-sm text-slate-400">palabras/s</span>
          </div>
          <p class="mt-1 text-xs text-slate-400">
            {tps(est.genTokS)} tokens/s · {PLACEMENT_LABEL[est.placement]}
            {#if est.genTokS > 0}· {readingMultiple(est.genTokS) >= 1
                ? `${readingMultiple(est.genTokS).toFixed(readingMultiple(est.genTokS) < 3 ? 1 : 0).replace('.', ',')}× tu velocidad de lectura`
                : 'más lento de lo que lees'}{/if}
          </p>
          <div class="mt-3 h-1 rounded-full bg-white/5">
            <div class="h-1 rounded-full bg-gradient-to-r from-sky-400 to-lime-300" style:width="{pct}%"></div>
          </div>
        </header>
        <div class="relative hidden h-44 overflow-hidden p-4 text-sm leading-relaxed text-slate-300 md:block">
          {#if est.placement === 'nofit'}
            <p class="text-rose-300">No cabe en la memoria de este equipo.</p>
          {:else if elapsed > 0 && shown === 0}
            <p class="animate-pulse text-slate-500">Leyendo el prompt…</p>
          {:else}
            <!-- Se ancla abajo para que se vea siempre el final, como en un chat. -->
            <p class="absolute inset-x-4 bottom-4 whitespace-pre-line">
              {tokens.slice(0, shown).join('')}{#if running && shown < tokens.length}<span class="animate-pulse text-sky-300">▍</span>{/if}
            </p>
          {/if}
          <div class="pointer-events-none absolute inset-x-0 top-0 h-12 bg-gradient-to-b from-slate-950 to-transparent"></div>
        </div>
        <footer class="mt-auto border-t border-white/5 px-4 py-2 text-xs text-slate-400">
          {#if shown >= tokens.length}
            ✓ Respuesta completa en {(PROMPT_TOKENS / est.prefillTokS + tokens.length / est.genTokS).toFixed(1)} s
          {:else if est.genTokS > 0}
            Tardaría ≈ {(PROMPT_TOKENS / est.prefillTokS + tokens.length / est.genTokS).toFixed(0)} s en total
          {:else}
            —
          {/if}
        </footer>
      </article>
    {/each}
  </div>
  <p class="mt-4 text-center text-xs text-slate-400">
    Misma pregunta, mismo modelo ({model?.name}, Q4_K_M), tres equipos. Velocidades calculadas con el mismo motor que usa el analizador.
  </p>
</div>
