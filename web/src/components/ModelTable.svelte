<script lang="ts">
  import { $hardware as hardware } from '../lib/store.ts';
  import { recommend, RATING_LABEL, PLACEMENT_LABEL, type Estimate } from '../lib/estimate.ts';
  import { gb, params, tps, RATING_CLASS } from '../lib/format.ts';
  import type { Gpu, Model } from '../lib/types.ts';
  import { url } from '../lib/url.ts';

  let { models, gpus }: { models: Model[]; gpus: Gpu[] } = $props();

  let tag = $state('todos');
  let onlyFitting = $state(false);

  const tags = $derived(['todos', ...new Set(models.flatMap((m) => m.tags))]);
  const hw = $derived($hardware);
  const gpu = $derived(hw?.gpuId ? (gpus.find((g) => g.id === hw.gpuId) ?? null) : null);

  const rows = $derived.by(() => {
    if (!hw) return [] as { model: Model; est: Estimate }[];
    return models
      .filter((m) => tag === 'todos' || m.tags.includes(tag))
      .map((model) => ({ model, est: recommend(model, hw, gpu) }))
      .filter((r) => !onlyFitting || r.est.placement !== 'nofit')
      .sort((a, b) => b.est.genTokS - a.est.genTokS || a.model.paramsB - b.model.paramsB);
  });
</script>

<section class="card">
  <div class="flex flex-wrap items-end justify-between gap-3">
    <div>
      <h2 class="text-lg font-semibold text-slate-100">Modelos para tu equipo</h2>
      <p class="mt-1 text-sm text-slate-400">Cuantización recomendada y velocidad estimada de generación con llama.cpp (contexto 8K).</p>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      {#each tags as t}
        <button
          class="rounded-full border px-3 py-1 text-xs {tag === t ? 'border-sky-400 bg-sky-400/10 text-sky-200' : 'border-slate-700 text-slate-400 hover:border-slate-500'}"
          onclick={() => (tag = t)}>{t}</button
        >
      {/each}
      <label class="ml-2 flex items-center gap-2 text-xs text-slate-400">
        <input type="checkbox" bind:checked={onlyFitting} class="accent-sky-400" /> Solo los que caben
      </label>
    </div>
  </div>

  {#if !hw}
    <p class="mt-6 text-sm text-slate-400">Esperando a la detección del hardware…</p>
  {:else}
    <div class="mt-5 overflow-x-auto">
      <table class="w-full min-w-[720px] text-left text-sm">
        <thead class="text-xs uppercase tracking-wide text-slate-500">
          <tr class="border-b border-slate-800">
            <th class="py-2 pr-3 font-medium">Modelo</th>
            <th class="py-2 pr-3 font-medium">Tamaño</th>
            <th class="py-2 pr-3 font-medium">Cuantización</th>
            <th class="py-2 pr-3 font-medium">Memoria</th>
            <th class="py-2 pr-3 font-medium">Dónde se ejecuta</th>
            <th class="py-2 pr-3 text-right font-medium">tok/s</th>
            <th class="py-2 font-medium">Experiencia</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as { model, est } (model.id)}
            <tr class="border-b border-slate-800/60 hover:bg-slate-800/30">
              <td class="py-3 pr-3">
                <a href={url(`/modelos/${model.id}`)} class="font-medium text-slate-100 hover:text-sky-300">{model.name}</a>
                <div class="text-xs text-slate-500">{model.org} · {model.tags.join(', ')}</div>
              </td>
              <td class="py-3 pr-3 text-slate-300">
                {params(model.paramsB)}{#if model.activeParamsB}{' '}<span class="text-slate-500">({params(model.activeParamsB)} act.)</span>{/if}
              </td>
              <td class="py-3 pr-3 font-mono text-xs text-slate-300">{est.quant}</td>
              <td class="py-3 pr-3 text-slate-300">{gb(est.totalGB)}</td>
              <td class="py-3 pr-3 text-slate-400">
                {PLACEMENT_LABEL[est.placement]}{#if est.placement === 'hybrid'}{' '}<span class="text-slate-500">({Math.round(est.gpuFraction * 100)} % GPU)</span>{/if}
              </td>
              <td class="py-3 pr-3 text-right font-mono text-slate-100">{tps(est.genTokS)}</td>
              <td class="py-3">
                <span class="rounded-full border px-2 py-0.5 text-xs {RATING_CLASS[est.rating]}">{RATING_LABEL[est.rating]}</span>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</section>
