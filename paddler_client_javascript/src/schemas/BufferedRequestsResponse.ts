import { z } from "zod";

export const BufferedRequestsResponseSchema = z
  .object({
    buffered_requests_current: z.number(),
  })
  .strict();

export type BufferedRequestsResponse = z.infer<
  typeof BufferedRequestsResponseSchema
>;
