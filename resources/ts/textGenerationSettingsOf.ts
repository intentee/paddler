import { type BalancerDesiredState } from "@intentee/paddler-client/schemas/BalancerDesiredState";
import { type BalancerTextGenerationSettings } from "@intentee/paddler-client/schemas/BalancerTextGenerationSettings";

export function textGenerationSettingsOf(
  balancerDesiredState: BalancerDesiredState,
): BalancerTextGenerationSettings {
  if (!("TextGeneration" in balancerDesiredState.inference_settings)) {
    throw new Error("The desired state does not serve text generation");
  }

  return balancerDesiredState.inference_settings.TextGeneration;
}
