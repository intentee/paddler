import { z } from "zod";

export const ValidatedParametersSchemaSchema = z
  .object({
    type: z.string(),
    properties: z.record(z.string(), z.unknown()).optional(),
    required: z.array(z.string()).optional(),
    additionalProperties: z.unknown().optional(),
  })
  .strict();

export type ValidatedParametersSchema = z.infer<
  typeof ValidatedParametersSchemaSchema
>;
