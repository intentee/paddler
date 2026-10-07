import { type BalancerDesiredState } from "@intentee/paddler-client/schemas/BalancerDesiredState";
import { type EmbeddingParameters } from "@intentee/paddler-client/schemas/EmbeddingParameters";

export function embeddingParametersOf(
  balancerDesiredState: BalancerDesiredState,
): EmbeddingParameters {
  if (!("Embeddings" in balancerDesiredState.inference_settings)) {
    throw new Error("The desired state does not serve embeddings");
  }

  return balancerDesiredState.inference_settings.Embeddings;
}
