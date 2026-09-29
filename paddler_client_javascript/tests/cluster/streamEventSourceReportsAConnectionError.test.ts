import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";
import { firstValueFrom, take, toArray } from "rxjs";

import { eventSourceConnectionErrorState } from "../../src/EventSourceConnectionErrorState";
import { eventSourceInitialState } from "../../src/EventSourceInitialState";
import { AgentsResponseSchema } from "../../src/schemas/AgentsResponse";
import { streamEventSource } from "../../src/streamEventSource";
import {
  PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
  SpawnedBalancer,
} from "../SpawnedBalancer";

test("streamEventSource reports a connection error", async function () {
  const balancer = await SpawnedBalancer.spawn({
    bufferedRequestTimeoutMilliseconds:
      PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
  });

  await balancer.process.terminate();

  deepStrictEqual(
    await firstValueFrom(
      streamEventSource({
        schema: AgentsResponseSchema,
        url: `http://${balancer.addresses.management}/api/v1/agents/stream`,
      }).pipe(take(2), toArray()),
    ),
    [eventSourceInitialState, eventSourceConnectionErrorState],
  );
});
