import type { ChildProcess } from "node:child_process";
import { once } from "node:events";
import { setTimeout } from "node:timers/promises";
import { z } from "zod";

import { ProcessDidNotExitError } from "./ProcessDidNotExitError";
import { ProcessExitedWithFailureError } from "./ProcessExitedWithFailureError";

const TRZCINA_DEFAULT_COOPERATIVE_DEADLINE_MILLISECONDS = 10_000;
const TRZCINA_DEFAULT_ABORT_DEADLINE_MILLISECONDS = 10_000;
const PADDLER_SHUTDOWN_BOUND_MILLISECONDS =
  TRZCINA_DEFAULT_COOPERATIVE_DEADLINE_MILLISECONDS +
  TRZCINA_DEFAULT_ABORT_DEADLINE_MILLISECONDS;

export class SpawnedProcess {
  readonly pid: number;
  readonly #childProcess: ChildProcess;
  readonly #exited: Promise<unknown>;

  constructor(childProcess: ChildProcess) {
    this.pid = z.number().parse(childProcess.pid);
    this.#childProcess = childProcess;
    this.#exited = once(childProcess, "exit");
  }

  async exitCode(): Promise<number | null> {
    await this.#exited;

    return this.#childProcess.exitCode;
  }

  async terminate(): Promise<void> {
    if (
      this.#childProcess.exitCode === null &&
      this.#childProcess.signalCode === null
    ) {
      this.#childProcess.kill("SIGTERM");
    }

    const exitedWithinShutdownBound = await Promise.race([
      this.#exited.then(function () {
        return true;
      }),
      setTimeout(PADDLER_SHUTDOWN_BOUND_MILLISECONDS, false, { ref: false }),
    ]);

    if (!exitedWithinShutdownBound) {
      this.#childProcess.kill("SIGKILL");
      await this.#exited;

      throw new ProcessDidNotExitError(this.pid);
    }

    if (this.#childProcess.exitCode !== 0) {
      throw new ProcessExitedWithFailureError(
        this.pid,
        this.#childProcess.exitCode,
        this.#childProcess.signalCode,
      );
    }
  }
}
