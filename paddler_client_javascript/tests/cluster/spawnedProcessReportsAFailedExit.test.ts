import { rejects } from "node:assert/strict";
import { spawn } from "node:child_process";
import { test } from "node:test";

import { CLAP_USAGE_ERROR_EXIT_CODE } from "../clapUsageErrorExitCode";
import { paddlerBinaryPath } from "../paddlerBinaryPath";
import { ProcessExitedWithFailureError } from "../ProcessExitedWithFailureError";
import { SpawnedProcess } from "../SpawnedProcess";

test("spawned process reports a failed exit", async function () {
  const spawnedProcess = new SpawnedProcess(
    spawn(paddlerBinaryPath(), ["--flag-paddler-does-not-have"], {
      stdio: ["ignore", "ignore", "ignore"],
    }),
  );

  await spawnedProcess.exitCode();

  await rejects(spawnedProcess.terminate(), function (error: unknown) {
    return (
      error instanceof ProcessExitedWithFailureError &&
      error.exitCode === CLAP_USAGE_ERROR_EXIT_CODE
    );
  });
});
