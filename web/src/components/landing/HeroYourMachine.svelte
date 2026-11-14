<script lang="ts">
  import { onMount } from 'svelte';
  import { $hardware as hardware, setHardware } from '../../lib/store.ts';
  import { detectInBrowser, fetchAgentReport, profileFromAgent, profileFromBrowser } from '../../lib/hardware.ts';
  import { recommend } from '../../lib/estimate.ts';
  import { readingMultiple, wps } from '../../lib/format.ts';
  import { url } from '../../lib/url.ts';
  import type { Gpu, Model } from '../../lib/types.ts';

  let { models, gpus }: { models: Model[]; gpus: Gpu[] } = $props();

  // Solo tras montar: el HTML prerenderizado no conoce el equipo del visitante.
  let mounted = $state(false);

  onMount(async () => {
    mounted = true;
    const report = await fetchAgentReport();
    if (report) return setHardware(profileFromAgent(report, gpus));
    if (!hardware.get()) setHardware(profileFromBrowser(await detectInBrowser(), gpus));
  });

  const hw = $derived(mounted ? $hardware : null);
  const gpu = $derived(hw?.gpuId ? (gpus.find((g) => g.id === hw.gpuId) ?? null) : null);
  const identified = $derived(!!hw && (!!gpu || hw.source !== 'browser'));

  /** El modelo más grande que va fluido (≥ 15 tok/s); si ninguno, el más rápido. */
  const pick = $derived.by(() => {
    if (!hw) return null;
    const all = models.map((m) => ({ model: m, est: recommend(m, hw, gpu) })).filter((r) => r.est.placement !== 'nofit');
    const fluid = all.filter((r) => r.est.genTokS >= 15).sort((a, b) => b.model.paramsB - a.model.paramsB);
    if (fluid[0]) return { ...fluid[0], fluid: true };
    const fastest = all.sort((a, b) => b.est.genTokS - a.est.genTokS)[0];
    return fastest ? { ...fastest, fluid: false } : null;
  });
</script>

<div class="mt-8 min-h-[92px]" aria-live="polite">
  {#if !hw}
    <div class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.03] px-4 py-3 text-sm text-slate-400">
      <span class="h-2 w-2 animate-pulse rounded-full bg-sky-400"></span>
      Analizando tu equipo…
    </div>
  {:else if identified && pick}
    <a
      href={url(`/modelos/${pick.model.id}`)}
      class="group block rounded-xl border border-sky-400/25 bg-sky-400/[0.06] px-4 py-3 text-sm transition-colors hover:border-sky-400/50"
    >
      <span class="block text-xs uppercase tracking-wider text-sky-300/80">
        Tu equipo · {gpu?.name ?? 'sin GPU dedicada'}{hw.source === 'agent' ? ' (agente)' : ''}
      </span>
      <span class="mt-1 block text-slate-300">
        {pick.fluid ? 'El modelo más capaz que te irá fluido es' : 'El modelo que mejor te irá es'}
        <strong class="text-slate-50">{pick.model.name}</strong>, a
        <strong class="text-lime-300">{wps(pick.est.genTokS)} palabras/s</strong>
        {#if readingMultiple(pick.est.genTokS) >= 1.5}({Math.round(readingMultiple(pick.est.genTokS))}× más rápido de lo que lees){/if}.
        <span class="text-sky-300 group-hover:underline">Ver detalles →</span>
      </span>
    </a>
  {:else}
    <a
      href={url('/analizar')}
      class="block rounded-xl border border-amber-400/25 bg-amber-400/[0.05] px-4 py-3 text-sm text-slate-300 transition-colors hover:border-amber-400/50"
    >
      Tu navegador no deja ver qué tarjeta gráfica tienes.
      <span class="text-amber-200">Indícala en el analizador</span> y verás a qué velocidad te irá cada modelo →
    </a>
  {/if}
</div>
