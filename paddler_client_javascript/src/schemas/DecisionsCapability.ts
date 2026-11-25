import { z } from "zod";

import { DecisionsUnsupportedReasonSchema } from "./DecisionsUnsupportedReason";

export const DecisionsCapabilitySchema = z.union([
  z
    .object({
      Supported: z.object({ reads_images: z.boolean() }).strict(),
    })
    .strict(),
  z.object({ Unsupported: DecisionsUnsupportedReasonSchema }).strict(),
]);

export type DecisionsCapability = z.infer<typeof DecisionsCapabilitySchema>;
