import type { BalancerAddresses } from "./BalancerAddresses";

export function inferenceSocketUrl({ inference }: BalancerAddresses): string {
  return `ws://${inference}/api/v1/inference_socket`;
}
