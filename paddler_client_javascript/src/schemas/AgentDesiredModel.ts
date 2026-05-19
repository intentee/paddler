import { z } from "zod";

import { HuggingFaceModelReferenceSchema } from "./HuggingFaceModelReference";
import { UrlModelReferenceSchema } from "./UrlModelReference";

export const AgentDesiredModelSchema = z.union([
  z.object({
    HuggingFace: HuggingFaceModelReferenceSchema,
  }),
  z.object({
    LocalToAgent: z.string(),
  }),
  z.object({
    Url: UrlModelReferenceSchema,
  }),
  z.literal("None"),
]);

export type AgentDesiredModel = z.infer<typeof AgentDesiredModelSchema>;
