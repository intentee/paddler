import { Observable } from "rxjs";
import type { z } from "zod";

import { eventSourceConnectedState } from "./EventSourceConnectedState";
import { eventSourceConnectionErrorState } from "./EventSourceConnectionErrorState";
import { eventSourceInitialState } from "./EventSourceInitialState";
import { eventSourceMessageState } from "./eventSourceMessageState";
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

    eventSource.addEventListener(
      "message",
      function ({ data }: MessageEvent<string>) {
        subscriber.next(eventSourceMessageState({ data, schema }));
      },
    );

    return function () {
      eventSource.close();
    };
  });
}
