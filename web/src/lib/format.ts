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
