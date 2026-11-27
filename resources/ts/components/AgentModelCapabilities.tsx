import React from "react";

import { type AgentRuntimeStatus } from "@intentee/paddler-client/schemas/AgentRuntimeStatus";
import { modelCapabilityKinds } from "../modelCapabilityKinds";
import { ModelCapabilityBadge } from "./ModelCapabilityBadge";

import { modelCapabilityBadges } from "./modelCapabilityBadges.module.css";

export function AgentModelCapabilities({
  runtime,
}: {
  runtime: AgentRuntimeStatus;
}) {
  if ("Idle" === runtime) {
    return (
      <div className={modelCapabilityBadges}>
        <i>Serves no model yet</i>
      </div>
    );
  }

  return (
    <div className={modelCapabilityBadges}>
      {modelCapabilityKinds.map(function (modelCapabilityKind) {
        return (
          <ModelCapabilityBadge
            key={modelCapabilityKind.label}
            label={modelCapabilityKind.label}
            status={modelCapabilityKind.statusOf(runtime.Serving.capabilities)}
          />
        );
      })}
    </div>
  );
}
