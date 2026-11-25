import { z } from "zod";

import { TextGenerationUnsupportedReasonSchema } from "./TextGenerationUnsupportedReason";

export const TextGenerationCapabilitySchema = z.union([
  z.literal("Supported"),
  z.object({ Unsupported: TextGenerationUnsupportedReasonSchema }).strict(),
]);

export type TextGenerationCapability = z.infer<
  typeof TextGenerationCapabilitySchema
>;
