import { z } from "zod";

const InvalidArgumentsJsonSchema = z.strictObject({
  column: z.number(),
  line: z.number(),
  message: z.string(),
});

export const ToolCallArgumentsSchema = z.union([
  z.strictObject({
    InvalidJson: z.strictObject({
      error: InvalidArgumentsJsonSchema,
      raw: z.string(),
    }),
  }),
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
