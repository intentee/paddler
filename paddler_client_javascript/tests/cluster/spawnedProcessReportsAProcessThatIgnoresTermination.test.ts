import { rejects } from "node:assert/strict";
import { test } from "node:test";

import { ProcessDidNotExitError } from "../ProcessDidNotExitError";
import {
  PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
  SpawnedBalancer,
} from "../SpawnedBalancer";

test("spawned process reports a process that ignores termination", async function () {
  const balancer = await SpawnedBalancer.spawn({
    bufferedRequestTimeoutMilliseconds:
      PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    inferenceMode: "TextGeneration",
  });

  process.kill(balancer.process.pid, "SIGSTOP");

  await rejects(balancer.process.terminate(), function (error: unknown) {
    return (
      error instanceof ProcessDidNotExitError &&
      error.pid === balancer.process.pid
    );
  });
});
