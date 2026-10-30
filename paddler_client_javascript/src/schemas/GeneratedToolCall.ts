import { z } from "zod";

export const ToolCallArgumentsSchema = z.union([
  z.strictObject({ InvalidJson: z.string() }),
  z.strictObject({ ValidJson: z.unknown() }),
]);

export const GeneratedToolCallSchema = z.object({
  arguments: ToolCallArgumentsSchema,
  id: z.string(),
  index: z.number(),
  name: z.string(),
});

export type GeneratedToolCall = z.infer<typeof GeneratedToolCallSchema>;
export type ToolCallArguments = z.infer<typeof ToolCallArgumentsSchema>;
