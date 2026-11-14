<script lang="ts">
  import { onMount } from 'svelte';
  import { recommend, RATING_LABEL } from '../../lib/estimate.ts';
  import { DEMO_MACHINES, tokenize } from '../../lib/demo.ts';
  import { tps, RATING_CLASS } from '../../lib/format.ts';
  import type { Gpu, Model } from '../../lib/types.ts';

  let { models, gpus }: { models: Model[]; gpus: Gpu[] } = $props();

  const SHOWN = ['llama-3.2-3b', 'qwen3-8b', 'gpt-oss-20b', 'qwen3-32b'];
  const ROTATE_MS = 6000;
  const TEXT = tokenize('Claro, aquí tienes un resumen de tres puntos sobre cómo funciona la memoria en un modelo local…');

  let index = $state(2);
  let typed = $state(0);
  let paused = false;
  let timer: ReturnType<typeof setInterval> | undefined;
  let raf = 0;

  const machine = $derived(DEMO_MACHINES[index]);
  const gpu = $derived(machine.gpuId ? (gpus.find((g) => g.id === machine.gpuId) ?? null) : null);
  const rows = $derived(
    SHOWN.map((id) => models.find((m) => m.id === id))
      .filter((m): m is Model => !!m)
      .map((m) => ({ model: m, est: recommend(m, machine.hw, gpu) })),
  );
  // Escala logarítmica: 1 tok/s → 0 %, 200 tok/s → 100 %.
  const width = (t: number) => (t <= 0 ? 0 : Math.max(3, Math.min(100, (Math.log10(Math.max(t, 1)) / Math.log10(200)) * 100)));
  const chat = $derived(rows.find((r) => r.model.id === 'qwen3-8b')?.est);

  function stream() {
    cancelAnimationFrame(raf);
    typed = 0;
    const rate = chat?.genTokS ?? 0;
    if (rate <= 0) return;
    const start = performance.now();
    const tick = (now: number) => {
      typed = Math.min(TEXT.length, Math.floor(((now - start) / 1000) * rate));
      if (typed < TEXT.length) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
  }

  function select(i: number) {
    index = i;
    stream();
  }

  onMount(() => {
    stream();
    if (!window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      timer = setInterval(() => {
        if (!paused) select((index + 1) % DEMO_MACHINES.length);
      }, ROTATE_MS);
    }
    return () => {
      clearInterval(timer);
      cancelAnimationFrame(raf);
    };
  });
</script>

<div
  class="glow-border rounded-2xl"
  role="region"
  aria-label="Demostración del analizador"
  onmouseenter={() => (paused = true)}
  onmouseleave={() => (paused = false)}
>
  <div class="overflow-hidden rounded-2xl bg-slate-950/95 shadow-2xl shadow-sky-500/10">
    <!-- Barra de ventana -->
    <div class="flex items-center gap-2 border-b border-white/5 px-4 py-3">
      <span class="h-3 w-3 rounded-full bg-rose-400/70"></span>
      <span class="h-3 w-3 rounded-full bg-amber-300/70"></span>
      <span class="h-3 w-3 rounded-full bg-lime-400/70"></span>
      <span class="ml-3 font-mono text-xs text-slate-500">ail-helper · análisis</span>
    </div>

    <!-- Selector de equipo -->
    <div class="flex gap-1 border-b border-white/5 p-2">
      {#each DEMO_MACHINES as m, i}
        <button
          class="min-w-0 flex-1 rounded-lg px-2 py-1.5 text-left text-xs transition-colors {i === index
            ? 'bg-white/[0.07] text-slate-100'
            : 'text-slate-500 hover:text-slate-300'}"
          onclick={() => select(i)}
        >
          <span class="block truncate font-medium">{m.label}</span>
          <span class="block truncate text-[10px] opacity-70">{m.detail}</span>
        </button>
      {/each}
    </div>

    <!-- Modelos -->
    <ul class="space-y-3 p-4">
      {#each rows as { model, est } (model.id)}
        <li>
          <div class="flex items-center justify-between text-xs">
            <span class="text-slate-300">{model.name} <span class="font-mono text-slate-600">{est.quant}</span></span>
            <span class="flex items-center gap-2">
              <span class="font-mono text-slate-100">{tps(est.genTokS)}{' '}<span class="text-slate-500">tok/s</span></span>
              <span class="rounded-full border px-1.5 py-px text-[10px] {RATING_CLASS[est.rating]}">{RATING_LABEL[est.rating]}</span>
            </span>
          </div>
          <div class="mt-1.5 h-1.5 rounded-full bg-white/5">
            <div
              class="h-1.5 rounded-full bg-gradient-to-r from-sky-500 to-lime-300 transition-[width] duration-700 ease-out"
              style:width="{width(est.genTokS)}%"
            ></div>
          </div>
        </li>
      {/each}
    </ul>

    <!-- Chat -->
    <div class="border-t border-white/5 p-4">
      <div class="flex items-center justify-between text-[10px] uppercase tracking-wider text-slate-500">
        <span>Simulación · Qwen3 8B</span>
        <span>{tps(chat?.genTokS ?? 0)} tok/s</span>
      </div>
      <p class="mt-2 h-12 overflow-hidden text-sm leading-relaxed text-slate-300">
        {TEXT.slice(0, typed).join('')}<span class="animate-pulse text-sky-300">▍</span>
      </p>
    </div>
  </div>
</div>
