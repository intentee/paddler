import { z } from "zod";

export const EmbeddingInputDocumentSchema = z.object({
  content: z.string(),
  id: z.string(),
});

export type EmbeddingInputDocument = z.infer<
  typeof EmbeddingInputDocumentSchema
>;
