import {
  filter,
  first,
  firstValueFrom,
  mergeMap,
  tap,
  timeout,
  type Observable,
} from "rxjs";

import type { EventSourceState } from "../src/EventSourceState";
import type { Agent } from "../src/schemas/Agent";
import type { AgentsResponseSchema } from "../src/schemas/AgentsResponse";
import { AgentReportedIssuesError } from "./AgentReportedIssuesError";
import { AgentsSnapshotUndeserializableError } from "./AgentsSnapshotUndeserializableError";
import { AgentsStreamClosedError } from "./AgentsStreamClosedError";
import { MODEL_LOAD_MILLISECONDS } from "./observationWindow";

export function waitForAgentReady({
  agentName,
  agentsStates,
  expectedSlotsTotal,
}: {
  agentName: string;
  agentsStates: Observable<EventSourceState<typeof AgentsResponseSchema>>;
  expectedSlotsTotal: number;
}): Promise<Agent> {
  return firstValueFrom(
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
      filter(function (agent) {
        return agent.name === agentName;
      }),
      tap(function (agent) {
        if (agent.issues.length > 0) {
          throw new AgentReportedIssuesError(agentName, agent.issues);
        }
      }),
      first(function (agent) {
        return (
          agent.state_application_status === "Applied" &&
          agent.slots_total === expectedSlotsTotal
        );
      }),
      timeout({ first: MODEL_LOAD_MILLISECONDS }),
    ),
  );
}
