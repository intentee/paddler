import { z } from "zod";

import { EmbeddingNormalizationMethodSchema } from "./EmbeddingNormalizationMethod";
import { PoolingTypeSchema } from "./PoolingType";

export const EmbeddingSchema = z.object({
  embedding: z.array(z.number()),
  normalization_method: EmbeddingNormalizationMethodSchema,
  pooling_type: PoolingTypeSchema,
  source_document_id: z.string(),
});

export type Embedding = z.infer<typeof EmbeddingSchema>;
