import { z } from "zod";

import { EmbeddingsUnsupportedReasonSchema } from "./EmbeddingsUnsupportedReason";

export const EmbeddingsCapabilitySchema = z.union([
  z.literal("Supported"),
  z.object({ Unsupported: EmbeddingsUnsupportedReasonSchema }).strict(),
]);

export type EmbeddingsCapability = z.infer<typeof EmbeddingsCapabilitySchema>;
