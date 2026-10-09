import {
  filter,
  first,
  firstValueFrom,
  from,
  map,
  merge,
  mergeMap,
  tap,
  type Observable,
} from "rxjs";

import type { EventSourceState } from "../src/EventSourceState";
import type { Agent } from "../src/schemas/Agent";
import type { AgentRuntimeStatus } from "../src/schemas/AgentRuntimeStatus";
import type { AgentsResponseSchema } from "../src/schemas/AgentsResponse";
import { AgentExitedBeforeReadyError } from "./AgentExitedBeforeReadyError";
import { AgentReportedIssuesError } from "./AgentReportedIssuesError";
import { AgentsSnapshotUndeserializableError } from "./AgentsSnapshotUndeserializableError";
import { AgentsStreamClosedError } from "./AgentsStreamClosedError";
import type { SpawnedProcess } from "./SpawnedProcess";

function slotsTotalOf(runtime: AgentRuntimeStatus): number {
  return runtime === "Idle" ? 0 : runtime.Serving.slots_total;
}

export function waitForAgentReady({
  agentName,
  agentProcess,
  agentsStates,
  expectedSlotsTotal,
}: {
  agentName: string;
  agentProcess: SpawnedProcess;
  agentsStates: Observable<EventSourceState<typeof AgentsResponseSchema>>;
  expectedSlotsTotal: number;
}): Promise<Agent> {
  return firstValueFrom(
    merge(
      agentsStates.pipe(
        mergeMap(function (agentsState) {
          if (agentsState.isConnectionError) {
            throw new AgentsStreamClosedError(agentName);
          }

          if (agentsState.isDeserializationError) {
            throw new AgentsSnapshotUndeserializableError(agentName);
          }

          return agentsState.isOk ? agentsState.data.agents : [];
        }),
      ),
      from(agentProcess.exitCode()).pipe(
        map(function (exitCode): never {
          throw new AgentExitedBeforeReadyError(agentName, exitCode);
        }),
      ),
    ).pipe(
      filter(function (agent) {
        return agent.name === agentName;
      }),
      tap(function (agent) {
        if (agent.status.issues.length > 0) {
          throw new AgentReportedIssuesError(agentName, agent.status.issues);
        }
      }),
      first(function (agent) {
        return (
          agent.status.state_application_status === "Applied" &&
          slotsTotalOf(agent.status.runtime) === expectedSlotsTotal
        );
      }),
    ),
  );
}
