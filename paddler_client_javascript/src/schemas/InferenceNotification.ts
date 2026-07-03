import { z } from "zod";

export const InferenceNotificationSchema = z.object({
  Notification: z.enum(["PromptingDisabled", "PromptingEnabled"]),
});

export type InferenceNotification = z.infer<
  typeof InferenceNotificationSchema
>["Notification"];
