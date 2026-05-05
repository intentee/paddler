import { useEffect, useState } from "react";
import type { z } from "zod";

import { eventSourceInitialState } from "@intentee/paddler-client/EventSourceInitialState";
import { type EventSourceState } from "@intentee/paddler-client/EventSourceState";
import { streamEventSource } from "@intentee/paddler-client/streamEventSource";

export function useEventSourceUpdates<TSchema extends z.ZodType>({
  endpoint,
  schema,
}: {
  endpoint: string;
  schema: TSchema;
}): EventSourceState<TSchema> {
  const [streamState, setEventSourceState] =
    useState<EventSourceState<TSchema>>(eventSourceInitialState);

  useEffect(
    function () {
      const subscription = streamEventSource({
        url: endpoint,
        schema,
      }).subscribe(setEventSourceState);

      return function () {
        subscription.unsubscribe();
      };
    },
    [endpoint, schema, setEventSourceState],
  );

  return streamState;
}
