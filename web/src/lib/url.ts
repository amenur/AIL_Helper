/** Ruta interna con el prefijo de despliegue (p. ej. /AIL_Helper en GitHub Pages). */
export function url(path: string): string {
  return import.meta.env.BASE_URL.replace(/\/$/, '') + path;
}
