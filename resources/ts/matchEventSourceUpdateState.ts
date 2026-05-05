import { type ReactNode } from "react";
import { z } from "zod";

import { type EventSourceConnectedState } from "@intentee/paddler-client/EventSourceConnectedState";
import { type EventSourceConnectionErrorState } from "@intentee/paddler-client/EventSourceConnectionErrorState";
import { type EventSourceDataSnapshotState } from "@intentee/paddler-client/EventSourceDataSnapshotState";
import { type EventSourceDeserializationErrorState } from "@intentee/paddler-client/EventSourceDeserializationErrorState";
import { type EventSourceInitialState } from "@intentee/paddler-client/EventSourceInitialState";
import { type EventSourceState } from "@intentee/paddler-client/EventSourceState";

interface Handlers<TSchema extends z.ZodType> {
  connected(state: EventSourceConnectedState): ReactNode;
  connectionError(state: EventSourceConnectionErrorState): ReactNode;
  dataSnapshot(state: EventSourceDataSnapshotState<TSchema>): ReactNode;
  deserializationError(state: EventSourceDeserializationErrorState): ReactNode;
  initial(state: EventSourceInitialState): ReactNode;
}

export function matchEventSourceUpdateState<TSchema extends z.ZodType>(
  streamState: EventSourceState<TSchema>,
  handlers: Handlers<NoInfer<TSchema>>,
): ReactNode {
  if (streamState.isInitial) {
    return handlers.initial(streamState);
  }

  if (streamState.isConnectionError) {
    return handlers.connectionError(streamState);
  }

  if (streamState.isDeserializationError) {
    return handlers.deserializationError(streamState);
  }

  if (streamState.isOk) {
    return handlers.dataSnapshot(streamState);
  }

  return handlers.connected(streamState);
}
