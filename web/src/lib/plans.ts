/**
 * Opciones sencillas para usuarios no técnicos: en vez de elegir entre modelos y
 * cuantizaciones, se elige entre "Rápido", "Equilibrado" y "Más inteligente".
 * Por debajo, cada opción se resuelve al mejor modelo del catálogo para ese equipo.
 */
import { defaultContext, estimate, type Estimate } from './estimate.ts';
import type { Gpu, HardwareProfile, Model } from './types.ts';

export type TierId = 'rapido' | 'equilibrado' | 'inteligente';

export interface TierDef {
  id: TierId;
  name: string;
  tagline: string;
  /** Candidatos por orden de preferencia. */
  candidates: string[];
  /** Por debajo de esta velocidad (tokens/s) no merece la pena ofrecerlo. */
  minTokS: number;
}

export const TIERS: TierDef[] = [
  {
    id: 'rapido',
    name: 'Rápido',
    tagline: 'El más ligero. Ideal para correos y textos cortos.',
    candidates: ['qwen3-4b', 'gemma-3-4b', 'llama-3.2-3b'],
    minTokS: 3,
  },
  {
    id: 'equilibrado',
    name: 'Equilibrado',
    tagline: 'Buena calidad para el día a día: redactar, resumir, traducir.',
    candidates: ['qwen3-8b', 'gpt-oss-20b', 'llama-3.1-8b'],
    minTokS: 5,
  },
  {
    id: 'inteligente',
    name: 'Más inteligente',
    tagline: 'Para documentos largos y tareas complejas. Pide un ordenador potente.',
    candidates: ['qwen3-30b-a3b', 'gpt-oss-20b', 'mistral-small-3.2-24b', 'qwen3-14b'],
    minTokS: 6,
  },
];

export interface TierOption {
  tier: TierDef;
  model: Model | null;
  est: Estimate | null;
  available: boolean;
}

/** Tokens de una respuesta típica (un párrafo largo, ~150 palabras). */
const ANSWER_TOKENS = 220;

export function answerSeconds(est: Estimate): number {
  return ANSWER_TOKENS / est.genTokS + 300 / est.prefillTokS;
}

/** "al momento", "unos 8 segundos"… para una respuesta típica. */
export function speedInWords(est: Estimate): string {
  const s = answerSeconds(est);
  if (s < 4) return 'al momento';
  if (s < 60) return `unos ${Math.round(s)} segundos`;
  return `más de ${Math.floor(s / 60)} minuto${s >= 120 ? 's' : ''}`;
}

/** Resuelve cada opción al mejor modelo que ese equipo puede mover. */
export function tierOptions(models: Model[], hw: HardwareProfile, gpu: Gpu | null): TierOption[] {
  // Dos opciones nunca pueden ser el mismo asistente.
  const used = new Set<string>();
  return TIERS.map((tier) => {
    const candidates = tier.candidates
      .map((id) => models.find((m) => m.id === id))
      .filter((m): m is Model => !!m)
      .map((model) => ({ model, est: estimate(model, model.defaultQuant, defaultContext(model), hw, gpu) }))
      .filter((c) => c.est.placement !== 'nofit' && c.est.genTokS >= tier.minTokS && !used.has(c.model.id));
    // En "Rápido" manda la velocidad; en las demás, el orden de calidad (el primero que va bien).
    const pick = tier.id === 'rapido' ? [...candidates].sort((a, b) => b.est.genTokS - a.est.genTokS)[0] : candidates[0];
    if (pick) used.add(pick.model.id);
    return { tier, model: pick?.model ?? null, est: pick?.est ?? null, available: !!pick };
  });
}

export type Verdict = 'great' | 'ok' | 'tight' | 'no';

export function recommendTier(options: TierOption[]): TierOption | null {
  const by = (id: TierId) => options.find((o) => o.tier.id === id && o.available);
  const smart = by('inteligente');
  const balanced = by('equilibrado');
  if (smart?.est && smart.est.genTokS >= 25) return smart;
  if (balanced?.est && balanced.est.genTokS >= 10) return balanced;
  return by('rapido') ?? balanced ?? smart ?? null;
}

export function verdict(options: TierOption[]): Verdict {
  const rec = recommendTier(options);
  if (!rec?.est) return 'no';
  if (rec.tier.id !== 'rapido') return 'great';
  // Por debajo de ~8 tokens/s, una respuesta normal tarda más de medio minuto.
  return rec.est.genTokS >= 8 ? 'ok' : 'tight';
}

export const VERDICT_TEXT: Record<Verdict, { title: string; text: string }> = {
  great: { title: 'Tu ordenador está preparado', text: 'Puede usar un asistente de buena calidad con respuestas rápidas.' },
  ok: { title: 'Tu ordenador puede usarlo', text: 'Irá bien con el asistente rápido, ideal para el día a día.' },
  tight: { title: 'Tu ordenador va justo', text: 'Funcionará, pero las respuestas tardarán. Con más memoria o una tarjeta gráfica iría mejor.' },
  no: { title: 'Tu ordenador no llega', text: 'No tiene memoria suficiente para un asistente local. Necesitarías al menos 8 GB de RAM.' },
};
