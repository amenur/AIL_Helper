<script lang="ts">
  import { onMount } from 'svelte';
  import { tokenize } from '../../lib/demo.ts';

  // Ejemplos de oficina, no técnicos.
  const EXAMPLES = [
    {
      ask: 'Redacta un correo para un cliente avisando de que su pedido llegará tres días tarde.',
      answer: `Asunto: Actualización sobre su pedido

Estimado/a cliente:

Le escribimos para informarle de que su pedido llegará con tres días de retraso respecto a la fecha prevista, debido a un problema con nuestro proveedor de transporte. Lamentamos las molestias y le confirmaremos la nueva fecha de entrega en cuanto la tengamos.

Gracias por su paciencia y su confianza.

Un saludo cordial.`,
    },
    {
      ask: 'Resume este contrato de alquiler en cinco puntos clave.',
      answer: `1. Duración: un año, prorrogable automáticamente salvo aviso con dos meses de antelación.
2. Renta: 950 € al mes, actualizable cada año según el IPC.
3. Fianza: dos mensualidades, que se devuelven al finalizar si no hay desperfectos.
4. Gastos: la comunidad corre a cargo del propietario; los suministros, del inquilino.
5. Obras: cualquier reforma necesita autorización por escrito.`,
    },
    {
      ask: 'Traduce al inglés: «Gracias por su visita. Le enviaremos el presupuesto mañana.»',
      answer: `"Thank you for your visit. We will send you the quote tomorrow."

Si lo prefieres más formal: "Thank you for visiting us. We will send you our quotation tomorrow."`,
    },
  ];

  let index = $state(0);
  let typed = $state(0);
  let raf = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const tokens = $derived(tokenize(EXAMPLES[index].answer));

  function play(i: number) {
    cancelAnimationFrame(raf);
    clearTimeout(timer);
    index = i;
    typed = 0;
    const total = tokenize(EXAMPLES[i].answer).length;
    const start = performance.now();
    const tick = (now: number) => {
      // Velocidad de un ordenador con tarjeta gráfica de gama media (~40 tokens/s).
      typed = Math.min(total, Math.floor(((now - start) / 1000) * 40));
      if (typed < total) raf = requestAnimationFrame(tick);
      else timer = setTimeout(() => play((i + 1) % EXAMPLES.length), 4500);
    };
    timer = setTimeout(() => (raf = requestAnimationFrame(tick)), 700);
  }

  onMount(() => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) typed = 9999;
    else play(0);
    return () => {
      cancelAnimationFrame(raf);
      clearTimeout(timer);
    };
  });
</script>

<div class="glow-border rounded-2xl">
  <div class="overflow-hidden rounded-2xl bg-slate-900 shadow-2xl shadow-sky-500/10">
    <div class="flex items-center gap-3 border-b border-white/5 bg-slate-950/60 px-4 py-3">
      <span class="flex gap-1.5">
        <span class="h-2.5 w-2.5 rounded-full bg-white/15"></span><span class="h-2.5 w-2.5 rounded-full bg-white/15"></span><span
          class="h-2.5 w-2.5 rounded-full bg-white/15"></span>
      </span>
      <span class="text-sm font-medium text-slate-300">Tu asistente</span>
      <span class="ml-auto flex items-center gap-1.5 text-xs text-lime-300">
        <span class="h-1.5 w-1.5 rounded-full bg-lime-400"></span> Privado · en tu ordenador
      </span>
    </div>
    <div class="space-y-4 p-5 text-sm">
      <div class="ml-auto w-fit max-w-[88%] rounded-2xl rounded-br-sm bg-sky-500/15 px-4 py-2.5 text-sky-50">
        {EXAMPLES[index].ask}
      </div>
      <div class="h-64 overflow-hidden rounded-2xl rounded-bl-sm bg-white/[0.05] px-4 py-3 leading-relaxed whitespace-pre-line text-slate-200">
        {tokens.slice(0, typed).join('')}{#if typed < tokens.length}<span class="animate-pulse text-sky-300">▍</span>{/if}
      </div>
      <div class="flex gap-2" role="tablist" aria-label="Ejemplos">
        {#each EXAMPLES as e, i}
          <button
            role="tab"
            aria-selected={i === index}
            aria-label={`Ejemplo ${i + 1}`}
            class="h-1.5 flex-1 rounded-full transition-colors {i === index ? 'bg-sky-400' : 'bg-white/10 hover:bg-white/25'}"
            onclick={() => play(i)}
          ></button>
        {/each}
      </div>
    </div>
  </div>
</div>
