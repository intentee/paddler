import { rejects } from "node:assert/strict";
import { test } from "node:test";
import { z } from "zod";

import { fetchJson } from "../../src/fetchJson";
import { HttpError } from "../../src/HttpError";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

const NOT_FOUND = 404;

test("fetchJson rejects an unsuccessful response", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ management }) {
      await rejects(
        fetchJson({
          schema: z.unknown(),
          signal: new AbortController().signal,
          url: `http://${management}/not-a-paddler-route`,
        }),
        function (error: unknown) {
          return error instanceof HttpError && error.statusCode === NOT_FOUND;
        },
      );
    },
  );
});
