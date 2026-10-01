import type { BalancerDesiredState } from "../src/schemas/BalancerDesiredState";
import { ALL_GPU_LAYERS } from "./allGpuLayers";

export function qwen3DesiredState(
  storedDesiredState: BalancerDesiredState,
): BalancerDesiredState {
  return {
    ...storedDesiredState,
    inference_parameters: {
      ...storedDesiredState.inference_parameters,
      min_p: 0,
      n_gpu_layers: ALL_GPU_LAYERS,
      penalty_frequency: 0,
      penalty_presence: 0,
      penalty_repeat: 1,
      temperature: 0,
      top_k: 1,
      top_p: 1,
    },
    model: {
      HuggingFace: {
        filename: "Qwen3-0.6B-Q8_0.gguf",
        repo_id: "Qwen/Qwen3-0.6B-GGUF",
        revision: "main",
      },
    },
  };
}
