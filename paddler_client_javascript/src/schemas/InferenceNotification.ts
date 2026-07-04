import { z } from "zod";

export const InferenceNotificationSchema = z.object({
  Notification: z.enum(["TokenGenerationDisabled", "TokenGenerationEnabled"]),
});

export type InferenceNotification = z.infer<
  typeof InferenceNotificationSchema
>["Notification"];
