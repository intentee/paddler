import React, { CSSProperties } from "react";

import { type Agent } from "@intentee/paddler-client/schemas/Agent";

import { agentListAgentStatus__progress } from "./AgentListAgentStatus.module.css";

export function AgentListAgentStatus({
  agent: {
    slots_processing,
    status: { desired_slots_total, runtime, state_application_status },
  },
}: {
  agent: Agent;
}) {
  const slots_total = runtime === "Idle" ? 0 : runtime.Serving.slots_total;

  switch (state_application_status) {
    case "Applied":
      return (
        <div
          className={agentListAgentStatus__progress}
          style={
            {
              "--slots-usage": `${slots_total > 0 ? (slots_processing / slots_total) * 100 : 0}%`,
            } as CSSProperties
          }
        >
          <progress
            value={slots_processing}
            max={slots_total}
            title={`${slots_processing} of ${slots_total} slots used`}
          />
          <abbr title="Slots processing / total / desired total">
            {slots_processing}/{slots_total}/{desired_slots_total}
          </abbr>
        </div>
      );
    case "AttemptedAndNotAppliable":
      return (
        <div className={agentListAgentStatus__progress}>
          <div>
            🏳️ <i>Needs your help</i>
          </div>
        </div>
      );
    case "AttemptedAndRetrying":
      return (
        <div className={agentListAgentStatus__progress}>
          <div>
            🤨 <i>Retrying</i>
          </div>
        </div>
      );
    case "Fresh":
      return (
        <div className={agentListAgentStatus__progress}>
          <div>
            ⏳ <i>Pending</i>
          </div>
        </div>
      );
    case "Stuck":
      return (
        <div className={agentListAgentStatus__progress}>
          <div>
            🤨 <i>Retrying, but seems stuck?</i>
          </div>
        </div>
      );
  }
}
