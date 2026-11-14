<script lang="ts">
  import { onMount } from 'svelte';
  import { recommend, RATING_LABEL } from '../../lib/estimate.ts';
  import { DEMO_ANSWER, DEMO_MACHINES, DEMO_QUESTION, tokenize, type DemoMachine } from '../../lib/demo.ts';
  import { READING_TOKS, wps, RATING_CLASS } from '../../lib/format.ts';
  import { $hardware as hardware } from '../../lib/store.ts';
  import type { Gpu, Model } from '../../lib/types.ts';

  let { models, gpus }: { models: Model[]; gpus: Gpu[] } = $props();

  const SHOWN = ['llama-3.2-3b', 'qwen3-8b', 'gpt-oss-20b', 'qwen3-32b'];
  const ROTATE_MS = 6000;
  // Misma pregunta que la carrera de la landing; aquí basta con el primer párrafo.
  const TEXT = tokenize(DEMO_ANSWER.split('\n\n')[0]);

  let index = $state(2);
  let mounted = $state(false);
  let pickedMine = false;

  /** El equipo del visitante, si lo conocemos con fiabilidad (GPU identificada o agente). */
  const mine = $derived.by((): DemoMachine | null => {
    const hw = mounted ? $hardware : null;
    if (!hw) return null;
    const g = hw.gpuId ? gpus.find((x) => x.id === hw.gpuId) : null;
    if (!g && hw.source === 'browser') return null;
    const vram = g && !g.unifiedMemory ? ` ${hw.vramGBOverride ?? g.vramGB} GB` : '';
    return {
      id: 'mine',
      label: 'Tu equipo',
      detail: `${g ? g.name.replace(/^(GeForce|Radeon) /, '') + vram : 'Sin GPU'} · ${hw.ramGB} GB`,
      gpuId: hw.gpuId,
      hw,
    };
  });
  const machines = $derived(mine ? [mine, ...DEMO_MACHINES] : DEMO_MACHINES);

  // Cuando aparece "Tu equipo", se selecciona una vez.
  $effect(() => {
    if (mine && !pickedMine) {
      pickedMine = true;
      select(0);
    }
  });
  let typed = $state(0);
  let paused = false;
  let timer: ReturnType<typeof setInterval> | undefined;
  let raf = 0;

  const machine = $derived(machines[Math.min(index, machines.length - 1)]);
  const gpu = $derived(machine.gpuId ? (gpus.find((g) => g.id === machine.gpuId) ?? null) : null);
  const rows = $derived(
    SHOWN.map((id) => models.find((m) => m.id === id))
      .filter((m): m is Model => !!m)
      .map((m) => ({ model: m, est: recommend(m, machine.hw, gpu) })),
  );
  // Escala logarítmica: 1 tok/s → 0 %, 200 tok/s → 100 %.
  // (La marca de lectura humana se sitúa en la misma escala.)
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
    mounted = true;
    stream();
    if (!window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      timer = setInterval(() => {
        if (!paused) select((index + 1) % machines.length);
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
      {#each machines as m, i (m.id)}
        <button
          class="min-w-0 flex-1 rounded-lg px-2 py-1.5 text-left text-xs transition-colors {i === index
            ? m.id === 'mine'
              ? 'bg-sky-400/15 text-sky-100'
              : 'bg-white/[0.07] text-slate-100'
            : m.id === 'mine'
              ? 'text-sky-300/80 hover:text-sky-200'
              : 'text-slate-400 hover:text-slate-200'}"
          onclick={() => select(i)}
        >
          <span class="block truncate font-medium">{m.label}</span>
          <span class="block truncate text-[10px] opacity-75">{m.detail}</span>
        </button>
      {/each}
    </div>

    <!-- Modelos -->
    <div class="p-4">
    <ul class="relative space-y-3 pt-5">
      <!-- Referencia: velocidad de lectura de una persona -->
      <li
        aria-hidden="true"
        class="pointer-events-none absolute top-0 bottom-0 border-l border-dashed border-amber-300/50"
        style:left="{width(READING_TOKS)}%"
      >
        <span class="absolute -top-0.5 left-1 whitespace-nowrap text-[10px] text-amber-200/80">velocidad de lectura</span>
      </li>
      {#each rows as { model, est } (model.id)}
        <li>
          <div class="flex items-center justify-between gap-2 text-xs">
            <span class="truncate text-slate-300">{model.name}</span>
            <span class="flex shrink-0 items-center gap-2">
              <span class="font-mono text-slate-100">{wps(est.genTokS)}{' '}<span class="text-slate-400">palabras/s</span></span>
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
    </div>

    <!-- Chat -->
    <div class="border-t border-white/5 p-4">
      <div class="flex items-center justify-between text-[10px] uppercase tracking-wider text-slate-400">
        <span>Simulación · Qwen3 8B</span>
        <span>{wps(chat?.genTokS ?? 0)} palabras/s</span>
      </div>
      <p class="mt-3 ml-auto w-fit max-w-[90%] rounded-2xl rounded-br-sm bg-sky-500/15 px-3 py-1.5 text-xs text-sky-50">
        {DEMO_QUESTION}
      </p>
      <!-- Anclado abajo: siempre se ve lo último que se ha escrito, como en un chat. -->
      <div
        class="relative mt-2 h-[4.5rem] overflow-hidden text-sm leading-relaxed text-slate-300 [mask-image:linear-gradient(to_bottom,transparent,#000_1.5rem)]"
      >
        <p class="absolute inset-x-0 bottom-0">
          {TEXT.slice(0, typed).join('')}{#if typed < TEXT.length}<span class="animate-pulse text-sky-300">▍</span>{/if}
        </p>
      </div>
    </div>
  </div>
</div>
