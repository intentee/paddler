import { AgentsResponseSchema } from "@intentee/paddler-client/schemas/AgentsResponse";
import { useEventSourceUpdates } from "./useEventSourceUpdates";

export function useAgentsStream({
  managementAddr,
}: {
  managementAddr: string;
}) {
  return useEventSourceUpdates({
    schema: AgentsResponseSchema,
    endpoint: `//${managementAddr}/api/v1/agents/stream`,
  });
}
