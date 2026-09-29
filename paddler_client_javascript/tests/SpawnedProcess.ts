import type { ChildProcess } from "node:child_process";
import { once } from "node:events";
import { setTimeout } from "node:timers/promises";
import { z } from "zod";

import { RELEASE_MILLISECONDS } from "./observationWindow";
import { ProcessDidNotExitError } from "./ProcessDidNotExitError";
import { ProcessExitedWithFailureError } from "./ProcessExitedWithFailureError";

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

    const exitedWithinRelease = await Promise.race([
      this.#exited.then(function () {
        return true;
      }),
      setTimeout(RELEASE_MILLISECONDS, false, { ref: false }),
    ]);

    if (!exitedWithinRelease) {
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
