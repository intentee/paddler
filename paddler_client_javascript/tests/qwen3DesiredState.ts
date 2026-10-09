import type { BalancerDesiredState } from "../src/schemas/BalancerDesiredState";
import { ALL_GPU_LAYERS } from "./allGpuLayers";

export function qwen3DesiredState(
  storedDesiredState: BalancerDesiredState,
): BalancerDesiredState {
  return {
    ...storedDesiredState,
    inference_mode: "TextGeneration",
    model: {
      Uri: "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/blob/main/Qwen3-0.6B-Q8_0.gguf",
    },
    model_runtime_parameters: {
      ...storedDesiredState.model_runtime_parameters,
      n_gpu_layers: ALL_GPU_LAYERS,
    },
    text_generation: {
      ...storedDesiredState.text_generation,
      sampling_parameters: {
        min_p: 0,
        penalty_frequency: 0,
        penalty_last_n: 0,
        penalty_presence: 0,
        penalty_repeat: 1,
        temperature: 0,
        top_k: 1,
        top_p: 1,
      },
    },
  };
}
