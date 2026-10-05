import { Observable } from "rxjs";
import type { z } from "zod";

import { eventSourceConnectedState } from "./EventSourceConnectedState";
import { eventSourceConnectionErrorState } from "./EventSourceConnectionErrorState";
import { eventSourceDeserializationErrorState } from "./EventSourceDeserializationErrorState";
import { eventSourceInitialState } from "./EventSourceInitialState";
import type { EventSourceState } from "./EventSourceState";

export function streamEventSource<TSchema extends z.ZodType>({
  url,
  schema,
}: {
  url: URL | string;
  schema: TSchema;
}): Observable<EventSourceState<TSchema>> {
  return new Observable<EventSourceState<TSchema>>(function (subscriber) {
    subscriber.next(eventSourceInitialState);

    const eventSource = new EventSource(url);

    eventSource.addEventListener("open", function () {
      subscriber.next(eventSourceConnectedState);
    });

    eventSource.addEventListener("error", function () {
      subscriber.next(eventSourceConnectionErrorState);
    });

    eventSource.addEventListener("message", function (event) {
      if ("string" !== typeof event.data) {
        subscriber.next(eventSourceDeserializationErrorState);

        return;
      }

      let parsedJson: unknown;

      try {
        parsedJson = JSON.parse(event.data);
      } catch {
        subscriber.next(eventSourceDeserializationErrorState);

        return;
      }

      const result = schema.safeParse(parsedJson);

      if (!result.success) {
        subscriber.next(eventSourceDeserializationErrorState);

        return;
      }

      subscriber.next({
        data: result.data,
        isConnected: true,
        isConnectionError: false,
        isDeserializationError: false,
        isInitial: false,
        isOk: true,
      });
    });

    return function () {
      eventSource.close();
    };
  });
}
