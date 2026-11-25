import { z } from "zod";

export const ChatTemplateCapabilitiesSchema = z
  .object({
    system_role: z.boolean(),
    thinking: z.boolean(),
    tool_calling: z.enum(["OneCallPerTurn", "ParallelCalls", "Unsupported"]),
  })
  .strict();

export type ChatTemplateCapabilities = z.infer<
  typeof ChatTemplateCapabilitiesSchema
>;
