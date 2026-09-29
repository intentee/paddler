import type { z } from "zod";

import type { EventSourceDataSnapshotState } from "./EventSourceDataSnapshotState";
import {
  eventSourceDeserializationErrorState,
  type EventSourceDeserializationErrorState,
} from "./EventSourceDeserializationErrorState";

export function eventSourceMessageState<TSchema extends z.ZodType>({
  data,
  schema,
}: {
  data: string;
  schema: TSchema;
}):
  | EventSourceDataSnapshotState<TSchema>
  | EventSourceDeserializationErrorState {
  let parsedJson: unknown;

  try {
    parsedJson = JSON.parse(data);
  } catch {
    return eventSourceDeserializationErrorState;
  }

  const result = schema.safeParse(parsedJson);

  if (!result.success) {
    return eventSourceDeserializationErrorState;
  }

  return {
    data: result.data,
    isConnected: true,
    isConnectionError: false,
    isDeserializationError: false,
    isInitial: false,
    isOk: true,
  };
}
