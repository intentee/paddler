import { z } from "zod";

import { EmbeddingInputDocumentSchema } from "./EmbeddingInputDocument";
import { EmbeddingNormalizationMethodSchema } from "./EmbeddingNormalizationMethod";

export const GenerateEmbeddingBatchParamsSchema = z.object({
  input_documents: z.array(EmbeddingInputDocumentSchema),
  normalization_method: EmbeddingNormalizationMethodSchema,
});

export type GenerateEmbeddingBatchParams = z.infer<
  typeof GenerateEmbeddingBatchParamsSchema
>;
