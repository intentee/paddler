import type { z } from "zod";

import type { EventSourceConnectedState } from "./EventSourceConnectedState";
import type { EventSourceConnectionErrorState } from "./EventSourceConnectionErrorState";
import type { EventSourceDataSnapshotState } from "./EventSourceDataSnapshotState";
import type { EventSourceDeserializationErrorState } from "./EventSourceDeserializationErrorState";
import type { EventSourceInitialState } from "./EventSourceInitialState";

export type EventSourceState<TSchema extends z.ZodType> =
  | EventSourceConnectedState
  | EventSourceConnectionErrorState
  | EventSourceDataSnapshotState<TSchema>
  | EventSourceDeserializationErrorState
  | EventSourceInitialState;
