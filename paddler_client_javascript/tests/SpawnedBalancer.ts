import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

import type { InferenceMode } from "../src/schemas/InferenceMode";
import {
  BalancerAddressesSchema,
  type BalancerAddresses,
} from "./BalancerAddresses";
import { BalancerExitedBeforeAnnouncingError } from "./BalancerExitedBeforeAnnouncingError";
import { paddlerBinaryPath } from "./paddlerBinaryPath";
import { SpawnedProcess } from "./SpawnedProcess";

const EPHEMERAL_LOOPBACK_ADDRESS = "127.0.0.1:0";

const servingModeArguments: Readonly<
  Record<InferenceMode, ReadonlyArray<string>>
> = {
  Decision: ["decision", "--compat-typesafe-addr", EPHEMERAL_LOOPBACK_ADDRESS],
  Embeddings: ["embeddings"],
  TextGeneration: [
    "text-generation",
    "--compat-openai-addr",
    EPHEMERAL_LOOPBACK_ADDRESS,
  ],
};

export const PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS = 10_000;

export class SpawnedBalancer {
  constructor(
    public readonly addresses: BalancerAddresses,
    public readonly process: SpawnedProcess,
  ) {}

  static async spawn({
    bufferedRequestTimeoutMilliseconds,
    inferenceMode,
  }: {
    bufferedRequestTimeoutMilliseconds: number;
    inferenceMode: InferenceMode;
  }): Promise<SpawnedBalancer> {
    const childProcess = spawn(
      paddlerBinaryPath(),
      [
        "balancer",
        ...servingModeArguments[inferenceMode],
        "--inference-addr",
        EPHEMERAL_LOOPBACK_ADDRESS,
        "--management-addr",
        EPHEMERAL_LOOPBACK_ADDRESS,
        "--buffered-request-timeout",
        String(bufferedRequestTimeoutMilliseconds),
      ],
      { stdio: ["ignore", "pipe", "inherit"] },
    );
    const spawnedProcess = new SpawnedProcess(childProcess);

    for await (const announcement of createInterface({
      input: childProcess.stdout,
    })) {
      return new SpawnedBalancer(
        BalancerAddressesSchema.parse(JSON.parse(announcement)),
        spawnedProcess,
      );
    }

    throw new BalancerExitedBeforeAnnouncingError(
      await spawnedProcess.exitCode(),
    );
  }
}
