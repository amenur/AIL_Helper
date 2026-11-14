import { atom } from 'nanostores';
import type { HardwareProfile } from './types.ts';

const KEY = 'ail-helper:hardware';

function load(): HardwareProfile | null {
  try {
    const raw = globalThis.localStorage?.getItem(KEY);
    return raw ? (JSON.parse(raw) as HardwareProfile) : null;
  } catch {
    return null;
  }
}

/** Perfil de hardware compartido entre todas las islas y páginas. */
export const $hardware = atom<HardwareProfile | null>(load());

export function setHardware(hw: HardwareProfile) {
  $hardware.set(hw);
  try {
    globalThis.localStorage?.setItem(KEY, JSON.stringify(hw));
  } catch {
    /* almacenamiento no disponible: solo se pierde la persistencia */
  }
}
