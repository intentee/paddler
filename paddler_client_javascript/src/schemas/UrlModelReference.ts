import { z } from "zod";

export const UrlModelReferenceSchema = z.object({
  url: z.string(),
});

export type UrlModelReference = z.infer<typeof UrlModelReferenceSchema>;
