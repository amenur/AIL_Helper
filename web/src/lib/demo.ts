import type { HardwareProfile } from './types.ts';

/** Trocea un texto en "tokens" aproximados (~4 caracteres), suficiente para animar. */
export function tokenize(text: string): string[] {
  return text.match(/\s*[^\s]{1,4}|\s+/g) ?? [];
}

export interface DemoMachine {
  id: string;
  label: string;
  detail: string;
  gpuId: string | null;
  hw: HardwareProfile;
}

const profile = (cpuThreads: number, ramGB: number, gpuId: string | null): HardwareProfile => ({
  source: 'manual',
  cpuThreads,
  ramGB,
  ramType: 'ddr5',
  gpuId,
});

/** Equipos de ejemplo para las demos de la landing. */
export const DEMO_MACHINES: DemoMachine[] = [
  { id: 'laptop', label: 'Portátil normal', detail: 'Sin tarjeta gráfica · 16 GB de RAM', gpuId: null, hw: profile(16, 16, null) },
  { id: 'rtx4060', label: 'Con tarjeta gráfica', detail: 'Gama media (RTX 4060) · 32 GB', gpuId: 'rtx-4060', hw: profile(12, 32, 'rtx-4060') },
  { id: 'rtx4090', label: 'Ordenador potente', detail: 'Gama alta (RTX 4090) · 64 GB', gpuId: 'rtx-4090', hw: profile(24, 64, 'rtx-4090') },
];

/** Pregunta y respuesta de la carrera de velocidad de la landing. */
export const DEMO_QUESTION = 'Cuéntame la historia de la paella y cómo se prepara correctamente.';

export const DEMO_ANSWER = `La paella nace en los campos de arroz que rodean la Albufera de Valencia. El arroz se cultivaba allí desde la época andalusí, y los jornaleros lo cocinaban al aire libre, sobre fuego de leña, con lo que tenían a mano: pollo, conejo, verduras de la huerta y, a veces, caracoles. El nombre viene del recipiente: en valenciano, "paella" es la sartén ancha y plana, del latín "patella". A mediados del siglo XIX ya aparece en recetarios, y con el tiempo se convirtió en un símbolo de la cocina española.

Para prepararla correctamente, sigue estos pasos:

1. Calienta aceite de oliva en la paella y dora el pollo y el conejo troceados, con sal, a fuego medio.
2. Añade la judía verde (ferraura) y el garrofó, y rehógalos unos minutos.
3. Incorpora el tomate rallado y una pizca de pimentón, sin dejar que se queme.
4. Vierte el agua y deja que hierva unos veinte minutos para que el caldo coja sabor. Añade el azafrán y ajusta la sal.
5. Reparte el arroz (bomba o senia) por toda la paella y, a partir de aquí, no lo remuevas.
6. Cuece unos diez minutos a fuego fuerte y otros ocho a fuego suave, hasta que el arroz absorba el caldo.
7. Al final, sube el fuego un minuto para conseguir el socarrat, la capa tostada del fondo.
8. Deja reposar cinco minutos, tapada con un paño, y sírvela directamente de la paella.

Un último consejo: la auténtica paella valenciana no lleva chorizo ni cebolla.`;
