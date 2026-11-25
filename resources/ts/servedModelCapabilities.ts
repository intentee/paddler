import { type Agent } from "@intentee/paddler-client/schemas/Agent";
import { type ModelCapabilities } from "@intentee/paddler-client/schemas/ModelCapabilities";

export function servedModelCapabilities(
  agents: Array<Agent>,
): Array<ModelCapabilities> {
  return agents.flatMap(function ({ status: { runtime } }) {
    return "Idle" === runtime ? [] : [runtime.Serving.capabilities];
  });
}
