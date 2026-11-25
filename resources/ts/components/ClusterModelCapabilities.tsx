import React from "react";

import { type Agent } from "@intentee/paddler-client/schemas/Agent";
import { modelCapabilityKinds } from "../modelCapabilityKinds";
import { servedModelCapabilities } from "../servedModelCapabilities";
import { ModelCapabilityBadge } from "./ModelCapabilityBadge";

import { clusterModelCapabilities } from "./ClusterModelCapabilities.module.css";
import { modelCapabilityBadges } from "./modelCapabilityBadges.module.css";

export function ClusterModelCapabilities({ agents }: { agents: Array<Agent> }) {
  const servedCapabilities = servedModelCapabilities(agents);

  return (
    <div className={clusterModelCapabilities}>
      <p>What the cluster serves</p>
      <div className={modelCapabilityBadges}>
        {modelCapabilityKinds.map(function ({ label, statusOf }) {
          const supportingAgentsCount = servedCapabilities.filter(
            function (capabilities) {
              return statusOf(capabilities).isSupported;
            },
          ).length;

          return (
            <ModelCapabilityBadge
              key={label}
              label={`${label} ${supportingAgentsCount}/${agents.length}`}
              status={{
                description: `${supportingAgentsCount} of ${agents.length} agents serve a model that supports it.`,
                isSupported: supportingAgentsCount > 0,
              }}
            />
          );
        })}
      </div>
    </div>
  );
}
