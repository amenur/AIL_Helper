import type { Rating } from './estimate.ts';

export const gb = (n: number) => `${n < 10 ? n.toFixed(1) : Math.round(n)} GB`;
export const tps = (n: number) => (n <= 0 ? '—' : n < 10 ? n.toFixed(1) : Math.round(n).toString());
export const params = (b: number) => (b < 10 ? `${b.toFixed(1)}B` : `${Math.round(b)}B`);
export const ctxLabel = (c: number) => (c >= 1024 ? `${Math.round(c / 1024)}K` : String(c));

export const RATING_CLASS: Record<Rating, string> = {
  excelente: 'bg-lime-400/15 text-lime-300 border-lime-400/30',
  fluida: 'bg-emerald-400/15 text-emerald-300 border-emerald-400/30',
  usable: 'bg-sky-400/15 text-sky-300 border-sky-400/30',
  lenta: 'bg-amber-400/15 text-amber-300 border-amber-400/30',
  'muy-lenta': 'bg-orange-500/15 text-orange-300 border-orange-500/30',
  'no-cabe': 'bg-rose-500/15 text-rose-300 border-rose-500/30',
};

/** Palabras por token aproximadas en español con tokenizadores modernos. */
export const WORDS_PER_TOKEN = 0.7;
/** Lectura silenciosa de un adulto: ~240 palabras/min ≈ 4 palabras/s. */
export const READING_WORDS_PER_S = 4;
export const READING_TOKS = READING_WORDS_PER_S / WORDS_PER_TOKEN;

export const wordsPerS = (tokS: number) => tokS * WORDS_PER_TOKEN;
/** "~45" palabras/s, redondeado de forma legible. */
export const wps = (tokS: number) => {
  const w = wordsPerS(tokS);
  if (w <= 0) return '—';
  if (w < 10) return `~${w.toFixed(1).replace('.', ',')}`;
  return `~${Math.round(w / 5) * 5 || Math.round(w)}`;
};
/** Cuántas veces más rápido que la lectura humana. */
export const readingMultiple = (tokS: number) => tokS / READING_TOKS;
