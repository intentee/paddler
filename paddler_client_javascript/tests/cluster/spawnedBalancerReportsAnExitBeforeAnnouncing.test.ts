import { rejects } from "node:assert/strict";
import { test } from "node:test";

import { BalancerExitedBeforeAnnouncingError } from "../BalancerExitedBeforeAnnouncingError";
import { SpawnedBalancer } from "../SpawnedBalancer";

const CLAP_USAGE_ERROR_EXIT_CODE = 2;

test("spawned balancer reports an exit before announcing", async function () {
  await rejects(
    SpawnedBalancer.spawn({ bufferedRequestTimeoutMilliseconds: -1 }),
    function (error: unknown) {
      return (
        error instanceof BalancerExitedBeforeAnnouncingError &&
        error.exitCode === CLAP_USAGE_ERROR_EXIT_CODE
      );
    },
  );
});
