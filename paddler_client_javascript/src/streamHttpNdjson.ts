import { Observable } from "rxjs";
import type { z } from "zod";

import { HttpError } from "./HttpError";
import { JsonError } from "./JsonError";

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
    fetch(url, {
      body: JSON.stringify(body),
      headers: { "Content-Type": "application/json" },
      method: "POST",
      signal,
    })
      .then(async function (response) {
        if (!response.ok) {
          throw new HttpError(
            response.status,
            `HTTP ${response.status} ${response.statusText}`,
          );
        }

        if (!response.body) {
          throw new HttpError(response.status, "Response has no body");
        }

        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buffer = "";

        while (!signal.aborted) {
          const { done, value } = await reader.read();

          if (done) {
            break;
          }

          buffer += decoder.decode(value, { stream: true });

          let newlineIndex = buffer.indexOf("\n");

          while (newlineIndex !== -1) {
            const line = buffer.slice(0, newlineIndex).trim();
            buffer = buffer.slice(newlineIndex + 1);

            if (line.length > 0) {
              let parsedJson: unknown;

              try {
                parsedJson = JSON.parse(line);
              } catch (error: unknown) {
                throw new JsonError(
                  `Failed to parse NDJSON line: ${String(error)}`,
                  line,
                );
              }

              subscriber.next(schema.parse(parsedJson));
            }

            newlineIndex = buffer.indexOf("\n");
          }
        }

        const trailing = buffer.trim();

        if (trailing.length > 0) {
          let parsedJson: unknown;

          try {
            parsedJson = JSON.parse(trailing);
          } catch (error: unknown) {
            throw new JsonError(
              `Failed to parse trailing NDJSON line: ${String(error)}`,
              trailing,
            );
          }

          subscriber.next(schema.parse(parsedJson));
        }

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
