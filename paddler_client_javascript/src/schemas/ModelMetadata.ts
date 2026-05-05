import { z } from "zod";

export const ModelMetadataSchema = z.record(z.string(), z.string());

export type ModelMetadata = z.infer<typeof ModelMetadataSchema>;
