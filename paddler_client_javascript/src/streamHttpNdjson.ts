import { Observable } from "rxjs";
import type { z } from "zod";

import { HttpError } from "./HttpError";
import { NdjsonDecoder } from "./NdjsonDecoder";

export function streamHttpNdjson<TSchema extends z.ZodType>({
  url,
  body,
  signal,
  schema,
}: {
  url: URL | string;
  body: unknown;
  signal: AbortSignal;
  schema: TSchema;
}): Observable<z.infer<TSchema>> {
  return new Observable(function (subscriber) {
    function emit(values: unknown[]): void {
      for (const value of values) {
        subscriber.next(schema.parse(value));
      }
    }

    fetch(url, {
      body: JSON.stringify(body),
      headers: { "Content-Type": "application/json" },
      method: "POST",
      signal,
    })
      .then(async function (response) {
        if (!response.ok || response.body === null) {
          throw new HttpError(
            response.status,
            `HTTP ${response.status} ${response.statusText}`,
          );
        }

        const reader = response.body.getReader();
        const textDecoder = new TextDecoder();
        const ndjsonDecoder = new NdjsonDecoder();

        while (!signal.aborted) {
          const { done, value } = await reader.read();

          if (done) {
            break;
          }

          emit(ndjsonDecoder.push(textDecoder.decode(value, { stream: true })));
        }

        emit(ndjsonDecoder.finish());
        subscriber.complete();
      })
      .catch(function (error: unknown) {
        if (signal.aborted) {
          subscriber.complete();

          return;
        }

        subscriber.error(error);
      });
  });
}
