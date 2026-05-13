import { z } from "zod";

export const PoolingTypeSchema = z.enum([
  "Cls",
  "Last",
  "Mean",
  "None",
  "Rank",
  "Unspecified",
]);

export type PoolingType = z.infer<typeof PoolingTypeSchema>;
