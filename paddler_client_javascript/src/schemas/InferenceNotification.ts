import { z } from "zod";

import { InferenceModeSchema } from "./InferenceMode";

export const InferenceNotificationSchema = z
  .object({
    Notification: z
      .object({
        ClusterInferenceMode: InferenceModeSchema,
      })
      .strict(),
  })
  .strict();

export type InferenceNotification = z.infer<
  typeof InferenceNotificationSchema
>["Notification"];
