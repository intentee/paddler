import { z } from "zod";

import { BalancerTextGenerationSettingsSchema } from "./BalancerTextGenerationSettings";
import { DecisionSettingsSchema } from "./DecisionSettings";
import { EmbeddingParametersSchema } from "./EmbeddingParameters";

export const BalancerInferenceSettingsSchema = z.union([
  z.object({ Decision: DecisionSettingsSchema }).strict(),
  z.object({ Embeddings: EmbeddingParametersSchema }).strict(),
  z.object({ TextGeneration: BalancerTextGenerationSettingsSchema }).strict(),
]);

export type BalancerInferenceSettings = z.infer<
  typeof BalancerInferenceSettingsSchema
>;
