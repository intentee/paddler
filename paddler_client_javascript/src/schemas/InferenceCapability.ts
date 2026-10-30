import { z } from "zod";

export const InferenceCapabilitySchema = z.enum([
  "Decisions",
  "Embeddings",
  "TextGeneration",
]);

export type InferenceCapability = z.infer<typeof InferenceCapabilitySchema>;
