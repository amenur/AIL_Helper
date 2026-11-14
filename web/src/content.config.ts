import { defineCollection } from 'astro:content';
import { file } from 'astro/loaders';
import { z } from 'astro/zod';

const models = defineCollection({
  loader: file('../shared/catalog/models.json'),
  schema: z.object({
    id: z.string(),
    name: z.string(),
    family: z.string(),
    org: z.string(),
    paramsB: z.number().positive(),
    activeParamsB: z.number().positive().optional(),
    layers: z.number().int().positive(),
    kvHeads: z.number().int().positive(),
    headDim: z.number().int().positive(),
    maxContext: z.number().int().positive(),
    kvCacheScale: z.number().positive().max(1).optional(),
    hfRepo: z.string().regex(/^[\w.-]+\/[\w.-]+$/),
    quants: z.array(z.string()).nonempty(),
    defaultQuant: z.string(),
    tags: z.array(z.string()),
    description: z.string(),
    license: z.string(),
  }),
});

const gpus = defineCollection({
  loader: file('../shared/catalog/gpus.json'),
  schema: z.object({
    id: z.string(),
    vendor: z.enum(['nvidia', 'amd', 'intel', 'apple']),
    name: z.string(),
    vramGB: z.number().nonnegative(),
    bandwidthGBs: z.number().positive(),
    fp16Tflops: z.number().positive(),
    backend: z.enum(['cuda', 'rocm', 'vulkan', 'metal']),
    laptop: z.boolean(),
    unifiedMemory: z.boolean().optional(),
  }),
});

export const collections = { models, gpus };
