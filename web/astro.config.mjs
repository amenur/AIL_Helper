// @ts-check
import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';
import tailwindcss from '@tailwindcss/vite';
import sitemap from '@astrojs/sitemap';

// En GitHub Pages el sitio vive en https://<usuario>.github.io/<repo>/.
// El workflow define SITE y BASE_PATH; en local se sirve desde la raíz.
export default defineConfig({
  site: process.env.SITE,
  base: process.env.BASE_PATH ?? '/',
  // La compresión de HTML se come el espacio entre un salto de línea y un elemento
  // ("del<strong>5 %</strong>" → "del5 %"). El ahorro es mínimo; se desactiva.
  compressHTML: false,
  // El sitemap necesita la URL absoluta del sitio, que solo existe en el despliegue.
  integrations: [svelte(), ...(process.env.SITE ? [sitemap({ filter: (page) => !page.includes('/vincular') })] : [])],
  vite: { plugins: [tailwindcss()] },
});
