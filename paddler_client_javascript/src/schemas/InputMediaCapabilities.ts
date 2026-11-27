import { z } from "zod";

export const InputMediaCapabilitiesSchema = z
  .object({
    audio: z.boolean(),
    image: z.boolean(),
  })
  .strict();

export type InputMediaCapabilities = z.infer<
  typeof InputMediaCapabilitiesSchema
>;
