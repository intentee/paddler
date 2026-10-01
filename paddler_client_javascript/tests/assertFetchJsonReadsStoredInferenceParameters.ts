import { deepStrictEqual } from "node:assert/strict";

import { fetchJson } from "../src/fetchJson";
import {
  BalancerDesiredStateSchema,
  type BalancerDesiredState,
} from "../src/schemas/BalancerDesiredState";
import type { InferenceParameters } from "../src/schemas/InferenceParameters";
import { putBalancerDesiredState } from "./putBalancerDesiredState";

export async function assertFetchJsonReadsStoredInferenceParameters({
  inferenceParameters,
  managementAddress,
}: {
  inferenceParameters: Partial<InferenceParameters>;
  managementAddress: string;
}): Promise<void> {
  function fetchDesiredState(): Promise<BalancerDesiredState> {
    return fetchJson({
      schema: BalancerDesiredStateSchema,
      signal: new AbortController().signal,
      url: `http://${managementAddress}/api/v1/balancer_desired_state`,
    });
  }

  const storedDesiredState = await fetchDesiredState();
  const updatedDesiredState = {
    ...storedDesiredState,
    inference_parameters: {
      ...storedDesiredState.inference_parameters,
      ...inferenceParameters,
    },
  };

  await putBalancerDesiredState({
    desiredState: updatedDesiredState,
    managementAddress,
  });

  deepStrictEqual(await fetchDesiredState(), updatedDesiredState);
}
