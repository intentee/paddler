import { z } from "zod";

import { ChatTemplateSchema } from "./ChatTemplate";

export const ChatTemplateSourceSchema = z.union([
  z
    .object({
      Override: ChatTemplateSchema,
    })
    .strict(),
  z.literal("EmbeddedInModel"),
]);

export type ChatTemplateSource = z.infer<typeof ChatTemplateSourceSchema>;
