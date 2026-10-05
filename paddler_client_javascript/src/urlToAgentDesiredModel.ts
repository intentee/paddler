import { extractHuggingFaceUrlParts } from "./extractHuggingFaceUrlParts";
import type { AgentDesiredModel } from "./schemas/AgentDesiredModel";
import { UnsupportedModelUrlError } from "./UnsupportedModelUrlError";

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

  if (url.protocol === "http:" || url.protocol === "https:") {
    return {
      Url: { url: url.toString() },
    };
  }

  throw new UnsupportedModelUrlError(url.toString());
}
