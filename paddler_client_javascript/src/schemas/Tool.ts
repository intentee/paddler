import { z } from "zod";

import { ValidatedParametersSchemaSchema } from "./ValidatedParametersSchema";

export const FunctionDefinitionSchema = z.object({
  name: z.string(),
  description: z.string(),
  parameters: ValidatedParametersSchemaSchema.optional(),
});

export const FunctionCallToolSchema = z.object({
  type: z.literal("function"),
  function: FunctionDefinitionSchema,
});

export const ToolSchema = FunctionCallToolSchema;

export type FunctionDefinition = z.infer<typeof FunctionDefinitionSchema>;
export type FunctionCallTool = z.infer<typeof FunctionCallToolSchema>;
export type Tool = z.infer<typeof ToolSchema>;
