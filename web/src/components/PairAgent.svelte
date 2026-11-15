<script lang="ts">
  import { onMount } from 'svelte';
  import { health, isPaired, isValidToken, setToken, wizardActive } from '../lib/agent.ts';
  import { url } from '../lib/url.ts';

  type State = 'checking' | 'paired' | 'invalid' | 'no-agent' | 'mismatch';
  let state = $state<State>('checking');

  onMount(async () => {
    const token = new URLSearchParams(location.hash.slice(1)).get('token') ?? '';
    // Se borra de la barra de direcciones y del historial.
    history.replaceState(null, '', location.pathname);
    if (!isValidToken(token)) {
      state = 'invalid';
      return;
    }
    setToken(token);
    if (!(await health())) {
      state = 'no-agent';
      return;
    }
    state = (await isPaired()) ? 'paired' : 'mismatch';
    // Si venía del asistente paso a paso, se continúa allí.
    if (state === 'paired' && wizardActive()) location.replace(url('/empezar'));
  });
</script>

<div class="card mx-auto max-w-xl text-center" aria-live="polite">
  {#if state === 'checking'}
    <p class="text-slate-300">Conectando con la app…</p>
  {:else if state === 'paired'}
    <p class="text-4xl">✓</p>
    <h1 class="mt-3 text-2xl font-bold text-slate-50">Conectado</h1>
    <p class="mt-2 text-slate-400">La app y esta página ya están conectadas en este navegador.</p>
    <a class="btn btn-lg mt-6" href={url('/empezar')}>Continuar →</a>
  {:else if state === 'no-agent'}
    <h1 class="text-2xl font-bold text-slate-50">La app no responde</h1>
    <p class="mt-2 text-slate-400">
      No encontramos la app abierta. Ábrela de nuevo y vuelve a esta página.
    </p>
  {:else if state === 'mismatch'}
    <h1 class="text-2xl font-bold text-slate-50">El código no coincide</h1>
    <p class="mt-2 text-slate-400">
      Cierra la app, ábrela otra vez y usa la pestaña que se abre.
    </p>
  {:else}
    <h1 class="text-2xl font-bold text-slate-50">Enlace no válido</h1>
    <p class="mt-2 text-slate-400">Esta página se abre sola al arrancar la app.</p>
  {/if}
</div>
