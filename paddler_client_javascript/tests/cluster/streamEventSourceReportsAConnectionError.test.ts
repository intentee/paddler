import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";
import { firstValueFrom, take, toArray } from "rxjs";

import { eventSourceConnectionErrorState } from "../../src/EventSourceConnectionErrorState";
import { eventSourceInitialState } from "../../src/EventSourceInitialState";
import { AgentsResponseSchema } from "../../src/schemas/AgentsResponse";
import { streamEventSource } from "../../src/streamEventSource";
import { agentsStreamUrl } from "../agentsStreamUrl";

const UNREACHABLE_ADDRESS = "127.0.0.1:1";

test("streamEventSource reports a connection error", async function () {
  deepStrictEqual(
    await firstValueFrom(
      streamEventSource({
        schema: AgentsResponseSchema,
        url: agentsStreamUrl(UNREACHABLE_ADDRESS),
      }).pipe(take(2), toArray()),
    ),
    [eventSourceInitialState, eventSourceConnectionErrorState],
  );
});
