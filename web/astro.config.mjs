// @ts-check
import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';
import tailwindcss from '@tailwindcss/vite';

// En GitHub Pages el sitio vive en https://<usuario>.github.io/<repo>/.
// El workflow define SITE y BASE_PATH; en local se sirve desde la raíz.
export default defineConfig({
  site: process.env.SITE,
  base: process.env.BASE_PATH ?? '/',
  integrations: [svelte()],
  vite: { plugins: [tailwindcss()] },
});
