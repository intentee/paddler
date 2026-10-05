import { deepStrictEqual } from "node:assert/strict";
import { once } from "node:events";
import { test } from "node:test";
import { lastValueFrom, toArray } from "rxjs";

import { inferenceSocketClient } from "../../src/inferenceSocketClient";
import { inferenceSocketUrl } from "../inferenceSocketUrl";
import { SHORT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../shortBufferedRequestTimeout";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

const GATEWAY_TIMEOUT = 504;

test("inferenceSocketClient delivers a timeout without agents", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        SHORT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function (addresses) {
      const webSocket = new WebSocket(inferenceSocketUrl(addresses));

      try {
        await once(webSocket, "open");

        const responses = await lastValueFrom(
          inferenceSocketClient({ webSocket })
            .continueConversation({
              enableThinking: false,
              messages: [{ content: "Hello", role: "user" }],
            })
            .pipe(toArray()),
        );

        deepStrictEqual(
          responses.map(function ({ done, error }) {
            return { done, errorCode: error?.code };
          }),
          [{ done: true, errorCode: GATEWAY_TIMEOUT }],
        );
      } finally {
        webSocket.close();
      }
    },
  );
});
