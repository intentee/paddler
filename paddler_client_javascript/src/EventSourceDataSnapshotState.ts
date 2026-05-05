import type { z } from "zod";

export type EventSourceDataSnapshotState<TSchema extends z.ZodType> = {
  data: z.infer<TSchema>;
  isConnected: true;
  isConnectionError: false;
  isDeserializationError: false;
  isInitial: false;
  isOk: true;
};
