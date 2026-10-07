import type { InferenceMode } from "../src/schemas/InferenceMode";
import type { BalancerAddresses } from "./BalancerAddresses";
import { SpawnedBalancer } from "./SpawnedBalancer";

export async function withSpawnedBalancer<TResult>(
  {
    bufferedRequestTimeoutMilliseconds,
    inferenceMode = "TextGeneration",
  }: {
    bufferedRequestTimeoutMilliseconds: number;
    inferenceMode?: InferenceMode;
  },
  body: (addresses: BalancerAddresses) => Promise<TResult>,
): Promise<TResult> {
  const balancer = await SpawnedBalancer.spawn({
    bufferedRequestTimeoutMilliseconds,
    inferenceMode,
  });

  try {
    return await body(balancer.addresses);
  } finally {
    await balancer.process.terminate();
  }
}
