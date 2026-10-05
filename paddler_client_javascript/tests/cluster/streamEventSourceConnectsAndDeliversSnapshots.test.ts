import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";
import { firstValueFrom, take, toArray } from "rxjs";

import { eventSourceConnectedState } from "../../src/EventSourceConnectedState";
import { eventSourceInitialState } from "../../src/EventSourceInitialState";
import { AgentsResponseSchema } from "../../src/schemas/AgentsResponse";
import { streamEventSource } from "../../src/streamEventSource";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

test("streamEventSource connects and delivers snapshots", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ management }) {
      deepStrictEqual(
        await firstValueFrom(
          streamEventSource({
            schema: AgentsResponseSchema,
            url: `http://${management}/api/v1/agents/stream`,
          }).pipe(take(3), toArray()),
        ),
        [
          eventSourceInitialState,
          eventSourceConnectedState,
          {
            data: { agents: [] },
            isConnected: true,
            isConnectionError: false,
            isDeserializationError: false,
            isInitial: false,
            isOk: true,
          },
        ],
      );
    },
  );
});
