import { z } from "zod";

export const DesiredStateVersionSchema = z.union([
  z.literal("Nothing"),
  z.object({ Version: z.number() }).strict(),
]);

export type DesiredStateVersion = z.infer<typeof DesiredStateVersionSchema>;
