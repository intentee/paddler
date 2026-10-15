import { z } from "zod";

export const AgentDesiredModelSchema = z.union([
  z.object({
    Uri: z.string(),
  }),
  z.literal("None"),
]);

export type AgentDesiredModel = z.infer<typeof AgentDesiredModelSchema>;
