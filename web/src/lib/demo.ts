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
  { id: 'laptop', label: 'Portátil sin GPU', detail: '16 hilos · 16 GB DDR5', gpuId: null, hw: profile(16, 16, null) },
  { id: 'rtx4060', label: 'PC gaming', detail: 'RTX 4060 8 GB · 32 GB', gpuId: 'rtx-4060', hw: profile(12, 32, 'rtx-4060') },
  { id: 'rtx4090', label: 'Workstation', detail: 'RTX 4090 24 GB · 64 GB', gpuId: 'rtx-4090', hw: profile(24, 64, 'rtx-4090') },
];

export const DEMO_ANSWER = `Para preparar una buena paella valenciana necesitas arroz bomba, pollo, conejo, judía verde, garrofó, tomate, aceite de oliva, azafrán y romero. Sofríe la carne a fuego medio hasta que se dore, añade las verduras y el tomate, y después el agua con el azafrán. Cuando hierva, reparte el arroz en forma de cruz, ajusta la sal y deja que se cocine sin removerlo durante unos dieciocho minutos. El secreto está en el socarrat: los últimos minutos a fuego fuerte.`;
