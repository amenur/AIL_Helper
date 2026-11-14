<script lang="ts">
  import { onMount } from 'svelte';
  import { health, isPaired, isValidToken, setToken } from '../lib/agent.ts';
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
  });
</script>

<div class="card mx-auto max-w-xl text-center" aria-live="polite">
  {#if state === 'checking'}
    <p class="text-slate-300">Vinculando con el agente local…</p>
  {:else if state === 'paired'}
    <p class="text-4xl">✓</p>
    <h1 class="mt-3 text-2xl font-bold text-slate-50">Agente vinculado</h1>
    <p class="mt-2 text-slate-400">
      Esta web ya puede pedir al agente que instale modelos y arranque el chat. Solo funciona en este navegador.
    </p>
    <a class="btn btn-lg mt-6" href={url('/analizar#modelos')}>Elegir un modelo →</a>
  {:else if state === 'no-agent'}
    <h1 class="text-2xl font-bold text-slate-50">El agente no responde</h1>
    <p class="mt-2 text-slate-400">
      Hemos guardado el código, pero no encontramos el agente en marcha. Ábrelo de nuevo y vuelve a esta página.
    </p>
  {:else if state === 'mismatch'}
    <h1 class="text-2xl font-bold text-slate-50">El código no coincide</h1>
    <p class="mt-2 text-slate-400">
      Usa el enlace que muestra la ventana del agente que tienes abierto ahora mismo.
    </p>
  {:else}
    <h1 class="text-2xl font-bold text-slate-50">Enlace no válido</h1>
    <p class="mt-2 text-slate-400">Abre esta página desde el enlace que muestra el agente al arrancar.</p>
  {/if}
</div>
