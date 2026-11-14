# AIL Helper

Local AI Helper: analiza tu equipo, estima a qué velocidad irá cada modelo de IA en local
con **llama.cpp** y ayuda a instalarlo junto con **Open WebUI** como interfaz de chat.

## Estructura

```
web/      Web en Astro + Svelte 5 + Tailwind 4 (catálogo, detección, simulador, instalación)
agent/    Agente local en Rust (pendiente): detección exacta, descargas y configuración
shared/
  catalog/  models.json y gpus.json: datos compartidos por la web y el agente
  schema/   Contrato JSON entre la web y el agente
```

## Web

```sh
cd web
npm install
npm run dev      # http://localhost:4321
npm test         # tests del motor de estimación
npm run check    # comprobación de tipos
npm run build    # sitio estático en web/dist
```

### Cómo estima la velocidad

- **Generación**: limitada por ancho de banda de memoria → `tok/s ≈ ancho de banda efectivo ÷ bytes leídos por token`.
  Con descarga parcial GPU + RAM se suman los tiempos de cada parte. En MoE solo cuentan los parámetros activos.
- **Lectura del prompt**: limitada por cómputo → `tok/s ≈ FLOPS efectivos ÷ (2 · parámetros activos)`.
- **Memoria**: pesos (`parámetros · bits por peso / 8`) + caché KV f16 + buffers.

Código en [`web/src/lib/estimate.ts`](web/src/lib/estimate.ts). Son estimaciones orientativas (±30 %).

### Añadir modelos o GPUs

Edita `shared/catalog/models.json` o `shared/catalog/gpus.json`. Astro valida ambos con Zod al compilar,
y `npm test` comprueba que las cuantizaciones existen y que los ids son únicos.

## Hoja de ruta

1. ✅ Web: catálogo, detección aproximada en el navegador, simulador e instrucciones de instalación.
2. Agente Rust: `GET /v1/hardware` con datos exactos.
3. Agente Rust: instalación automática de llama.cpp, del modelo y de Open WebUI.
4. Icono en la bandeja, arranque automático, versión de escritorio con Tauri.
