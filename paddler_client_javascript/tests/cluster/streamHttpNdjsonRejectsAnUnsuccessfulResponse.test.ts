import { rejects } from "node:assert/strict";
import { test } from "node:test";
import { firstValueFrom } from "rxjs";
import { z } from "zod";

import { HttpError } from "../../src/HttpError";
import { streamHttpNdjson } from "../../src/streamHttpNdjson";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

const NOT_FOUND = 404;

test("streamHttpNdjson rejects an unsuccessful response", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ inference }) {
      await rejects(
        firstValueFrom(
          streamHttpNdjson({
            body: {},
            schema: z.unknown(),
            signal: new AbortController().signal,
            url: `http://${inference}/not-a-paddler-route`,
          }),
        ),
        function (error: unknown) {
          return error instanceof HttpError && error.statusCode === NOT_FOUND;
        },
      );
    },
  );
});
