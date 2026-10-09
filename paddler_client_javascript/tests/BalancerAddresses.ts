import { z } from "zod";

export const BalancerAddressesSchema = z
  .object({
    compat_openai: z.string().nullable(),
    compat_typesafe: z.string().nullable(),
    inference: z.string(),
    management: z.string(),
    web_admin_panel: z.string().nullable(),
  })
  .strict();

export type BalancerAddresses = z.infer<typeof BalancerAddressesSchema>;
