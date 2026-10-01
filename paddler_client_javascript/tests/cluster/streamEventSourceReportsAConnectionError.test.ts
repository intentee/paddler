import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";
import { firstValueFrom, take, toArray } from "rxjs";

import { eventSourceConnectionErrorState } from "../../src/EventSourceConnectionErrorState";
import { eventSourceInitialState } from "../../src/EventSourceInitialState";
import { AgentsResponseSchema } from "../../src/schemas/AgentsResponse";
import { streamEventSource } from "../../src/streamEventSource";
import { withConnectionClosingListener } from "../withConnectionClosingListener";

test("streamEventSource reports a connection error", async function () {
  await withConnectionClosingListener(async function (address) {
    deepStrictEqual(
      await firstValueFrom(
        streamEventSource({
          schema: AgentsResponseSchema,
          url: `http://${address}/api/v1/agents/stream`,
        }).pipe(take(2), toArray()),
      ),
      [eventSourceInitialState, eventSourceConnectionErrorState],
    );
  });
});
