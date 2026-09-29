import { HttpError } from "../src/HttpError";
import type { BalancerDesiredState } from "../src/schemas/BalancerDesiredState";

export async function putBalancerDesiredState({
  desiredState,
  managementAddress,
}: {
  desiredState: BalancerDesiredState;
  managementAddress: string;
}): Promise<void> {
  const response = await fetch(
    `http://${managementAddress}/api/v1/balancer_desired_state`,
    {
      body: JSON.stringify(desiredState),
      headers: { "Content-Type": "application/json" },
      method: "PUT",
    },
  );

  if (!response.ok) {
    throw new HttpError(
      response.status,
      `HTTP ${response.status} ${response.statusText}`,
    );
  }
}
