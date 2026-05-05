import { useCallback } from "react";

import { BalancerDesiredStateSchema } from "@intentee/paddler-client/schemas/BalancerDesiredState";
import { useFetchJson } from "./useFetchJson";

export function useBalancerDesiredState({
  managementAddr,
}: {
  managementAddr: string;
}) {
  const produceFetchPromise = useCallback(
    function (signal: AbortSignal) {
      return fetch(`//${managementAddr}/api/v1/balancer_desired_state`, {
        signal,
      });
    },
    [managementAddr],
  );

  const result = useFetchJson({
    produceFetchPromise,
    responseSchema: BalancerDesiredStateSchema,
  });

  return result;
}
