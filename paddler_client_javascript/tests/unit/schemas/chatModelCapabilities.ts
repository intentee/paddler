import type { ModelCapabilities } from "../../../src/schemas/ModelCapabilities";

export const chatModelCapabilities: ModelCapabilities = {
  conversation: {
    Supported: {
      system_role: true,
      thinking: true,
      tool_calling: "ParallelCalls",
    },
  },
  decisions: { Unsupported: "ModelIsNotADecisionModel" },
  embeddings: { Unsupported: "ModelDeclaresNoVectorPooling" },
  input_media: { audio: false, image: false },
  model_defaults: {
    context_size: 4096,
    sampling: {
      min_p: 0.05,
      penalty_frequency: 0,
      penalty_last_n: 64,
      penalty_presence: 0,
      penalty_repeat: 1,
      temperature: 0.8,
      top_k: 40,
      top_p: 0.95,
    },
  },
  text_generation: "Supported",
};
