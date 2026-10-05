import { deepStrictEqual } from "node:assert/strict";
import { once } from "node:events";
import { test } from "node:test";
import { lastValueFrom, toArray } from "rxjs";

import { inferenceSocketClient } from "../../src/inferenceSocketClient";
import { inferenceSocketUrl } from "../inferenceSocketUrl";
import { qwen3DesiredState } from "../qwen3DesiredState";
import { withPaddlerCluster } from "../withPaddlerCluster";

test("inferenceSocketClient streams tokens until done", async function () {
  const responses = await withPaddlerCluster(
    {
      agents: [{ name: "qwen3-agent", slots: 1 }],
      configureDesiredState: qwen3DesiredState,
    },
    async function (addresses) {
      const webSocket = new WebSocket(inferenceSocketUrl(addresses));

      try {
        await once(webSocket, "open");

        return await lastValueFrom(
          inferenceSocketClient({ webSocket })
            .continueConversation({
              enableThinking: false,
              messages: [{ content: "Say hello.", role: "user" }],
            })
            .pipe(toArray()),
        );
      } finally {
        webSocket.close();
      }
    },
  );

  const tokens = responses.slice(0, -1);
  const done = responses.at(-1);

  deepStrictEqual(
    new Set(
      tokens.map(function ({ tokenKind }) {
        return tokenKind;
      }),
    ),
    new Set(["content"]),
  );
  deepStrictEqual({ done: done?.done, ok: done?.ok }, { done: true, ok: true });
});
