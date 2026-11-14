import { getCollection } from 'astro:content';
import type { Gpu, Model } from './types.ts';

export async function getModels(): Promise<Model[]> {
  return (await getCollection('models')).map((e) => e.data as Model);
}

export async function getGpus(): Promise<Gpu[]> {
  return (await getCollection('gpus')).map((e) => e.data as Gpu);
}
