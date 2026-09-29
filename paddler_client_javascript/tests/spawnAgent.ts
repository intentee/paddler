import { spawn } from "node:child_process";

import type { AgentSpec } from "./AgentSpec";
import { paddlerBinaryPath } from "./paddlerBinaryPath";
import { SpawnedProcess } from "./SpawnedProcess";

export function spawnAgent({
  agent: { name, slots },
  managementAddress,
}: {
  agent: AgentSpec;
  managementAddress: string;
}): SpawnedProcess {
  return new SpawnedProcess(
    spawn(
      paddlerBinaryPath(),
      [
        "agent",
        "--management-addr",
        managementAddress,
        "--name",
        name,
        "--slots",
        String(slots),
      ],
      { stdio: ["ignore", "ignore", "inherit"] },
    ),
  );
}
