import { z } from "zod";

export const ModelDownloadStatusSchema = z.union([
  z.object({
    Downloading: z
      .object({
        downloaded_bytes: z.number(),
        model_path: z.string(),
        total_bytes: z.number(),
      })
      .strict(),
  }),
  z.object({
    DownloadingWithUnknownSize: z
      .object({
        downloaded_bytes: z.number(),
        model_path: z.string(),
      })
      .strict(),
  }),
  z.literal("NotDownloading"),
]);

export type ModelDownloadStatus = z.infer<typeof ModelDownloadStatusSchema>;
