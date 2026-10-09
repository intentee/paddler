import { deepStrictEqual } from "node:assert/strict";

import { fetchJson } from "../src/fetchJson";
import {
  BalancerDesiredStateSchema,
  type BalancerDesiredState,
} from "../src/schemas/BalancerDesiredState";
import { putBalancerDesiredState } from "./putBalancerDesiredState";

export async function assertFetchJsonReadsStoredDesiredState({
  managementAddress,
  updateDesiredState,
}: {
  managementAddress: string;
  updateDesiredState(
    this: void,
    storedDesiredState: BalancerDesiredState,
  ): BalancerDesiredState;
}): Promise<void> {
  function fetchDesiredState(): Promise<BalancerDesiredState> {
    return fetchJson({
      schema: BalancerDesiredStateSchema,
      signal: new AbortController().signal,
      url: `http://${managementAddress}/api/v1/balancer_desired_state`,
    });
  }

  const updatedDesiredState = updateDesiredState(await fetchDesiredState());

  await putBalancerDesiredState({
    desiredState: updatedDesiredState,
    managementAddress,
  });

  deepStrictEqual(await fetchDesiredState(), updatedDesiredState);
}
