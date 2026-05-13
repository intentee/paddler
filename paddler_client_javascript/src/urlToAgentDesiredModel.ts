import { extractHuggingFaceUrlParts } from "./extractHuggingFaceUrlParts";
import type { AgentDesiredModel } from "./schemas/AgentDesiredModel";

export function urlToAgentDesiredModel(url: URL): AgentDesiredModel {
  if (url.hostname === "huggingface.co") {
    return {
      HuggingFace: extractHuggingFaceUrlParts(url),
    };
  }

  if (url.protocol === "agent:") {
    return {
      LocalToAgent: url.pathname,
    };
  }

  throw new Error("Unsupported URL format");
}
