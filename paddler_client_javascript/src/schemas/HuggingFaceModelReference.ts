import { z } from "zod";

export const HuggingFaceModelReferenceSchema = z.object({
  filename: z.string(),
  repo_id: z.string(),
  revision: z.string(),
});

export type HuggingFaceModelReference = z.infer<
  typeof HuggingFaceModelReferenceSchema
>;
