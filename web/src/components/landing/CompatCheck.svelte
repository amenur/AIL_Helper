<script lang="ts">
  import { onMount } from 'svelte';
  import { $hardware as hardware, setHardware } from '../../lib/store.ts';
  import { detectInBrowser, fetchAgentReport, profileFromAgent, profileFromBrowser } from '../../lib/hardware.ts';
  import { recommendTier, speedInWords, tierOptions, verdict, VERDICT_TEXT } from '../../lib/plans.ts';
  import { url } from '../../lib/url.ts';
  import type { Gpu, Model } from '../../lib/types.ts';

  let { models, gpus }: { models: Model[]; gpus: Gpu[] } = $props();

  let mounted = $state(false);
  let isMac = $state(false);

  onMount(async () => {
    mounted = true;
    isMac = /Mac OS X|Macintosh/i.test(navigator.userAgent);
    const report = await fetchAgentReport();
    if (report) return setHardware(profileFromAgent(report, gpus));
    if (!hardware.get()) setHardware(profileFromBrowser(await detectInBrowser(), gpus));
  });

  const hw = $derived(mounted ? $hardware : null);
  const gpu = $derived(hw?.gpuId ? (gpus.find((g) => g.id === hw.gpuId) ?? null) : null);
  // Sin GPU identificada y sin la app, el navegador no da datos fiables: no damos veredicto.
  const known = $derived(!!hw && (!!gpu || hw.source !== 'browser'));
  const options = $derived(hw ? tierOptions(models, hw, gpu) : []);
  const v = $derived(options.length ? verdict(options) : null);
  const rec = $derived(recommendTier(options));
</script>

<div class="mt-8 min-h-[76px]" aria-live="polite">
  {#if !hw}
    <p class="flex items-center gap-2 text-sm text-slate-400">
      <span class="h-2 w-2 animate-pulse rounded-full bg-sky-400"></span> Comprobando tu ordenador…
    </p>
  {:else if isMac}
    <p class="rounded-xl border border-white/10 bg-white/[0.03] px-4 py-3 text-sm text-slate-300">
      Por ahora funciona en <strong class="text-slate-100">Windows</strong>. La versión para Mac llegará pronto.
    </p>
  {:else if known && v}
    <a
      href={url('/empezar')}
      class="block rounded-xl border px-4 py-3 text-sm transition-colors {v === 'no'
        ? 'border-rose-400/30 bg-rose-400/[0.06]'
        : v === 'tight'
          ? 'border-amber-400/30 bg-amber-400/[0.06]'
          : 'border-lime-400/30 bg-lime-400/[0.06] hover:border-lime-400/60'}"
    >
      <strong class="text-slate-50">{v === 'no' ? '✕' : v === 'tight' ? '!' : '✓'} {VERDICT_TEXT[v].title}.</strong>
      <span class="text-slate-300">
        {#if rec?.est && v !== 'no'}
          Te recomendamos el asistente <strong class="text-slate-100">{rec.tier.name}</strong>: responde {speedInWords(rec.est)}.
        {:else}
          {VERDICT_TEXT[v].text}
        {/if}
      </span>
    </a>
  {:else}
    <p class="rounded-xl border border-white/10 bg-white/[0.03] px-4 py-3 text-sm text-slate-300">
      Tu navegador no nos deja ver bien tu ordenador. No pasa nada: <a class="text-sky-300 underline" href={url('/empezar')}>al empezar</a>, la app lo comprobará por ti.
    </p>
  {/if}
</div>
