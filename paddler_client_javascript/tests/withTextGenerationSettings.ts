import type { BalancerDesiredState } from "../src/schemas/BalancerDesiredState";
import type { BalancerTextGenerationSettings } from "../src/schemas/BalancerTextGenerationSettings";
import { DesiredStateServesAnotherModeError } from "./DesiredStateServesAnotherModeError";

export function withTextGenerationSettings(
  desiredState: BalancerDesiredState,
  updateTextGenerationSettings: (
    this: void,
    textGenerationSettings: BalancerTextGenerationSettings,
  ) => BalancerTextGenerationSettings,
): BalancerDesiredState {
  if (!("TextGeneration" in desiredState.inference_settings)) {
    throw new DesiredStateServesAnotherModeError();
  }

  return {
    ...desiredState,
    inference_settings: {
      TextGeneration: updateTextGenerationSettings(
        desiredState.inference_settings.TextGeneration,
      ),
    },
  };
}
