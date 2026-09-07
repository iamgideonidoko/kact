import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';
import { docsSchema, i18nSchema } from '@astrojs/starlight/schema';

export const collections = {
  docs: defineCollection({
    loader: glob({ base: './src/content/docs', pattern: '**/*.{md,mdx}' }),
    schema: docsSchema()
  }),
  i18n: defineCollection({
    loader: glob({ base: './src/content/i18n', pattern: '**/*.json' }),
    schema: i18nSchema()
  })
};
