import { z } from "zod";

export const ToolCallArgumentsSchema = z.union([
  z.strictObject({ InvalidJson: z.string() }),
  z.strictObject({ ValidJson: z.unknown() }),
]);

export const ParsedToolCallSchema = z.object({
  id: z.string(),
  name: z.string(),
  arguments: ToolCallArgumentsSchema,
});

export type ParsedToolCall = z.infer<typeof ParsedToolCallSchema>;
export type ToolCallArguments = z.infer<typeof ToolCallArgumentsSchema>;
