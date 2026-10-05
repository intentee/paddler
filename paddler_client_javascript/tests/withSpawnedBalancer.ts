import type { BalancerAddresses } from "./BalancerAddresses";
import { SpawnedBalancer } from "./SpawnedBalancer";

export async function withSpawnedBalancer<TResult>(
  {
    bufferedRequestTimeoutMilliseconds,
  }: {
    bufferedRequestTimeoutMilliseconds: number;
  },
  body: (addresses: BalancerAddresses) => Promise<TResult>,
): Promise<TResult> {
  const balancer = await SpawnedBalancer.spawn({
    bufferedRequestTimeoutMilliseconds,
  });

  try {
    return await body(balancer.addresses);
  } finally {
    await balancer.process.terminate();
  }
}
