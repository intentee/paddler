import { createContext } from "react";

export type PromptThinkingContextValue = {
  isThinkingEnabled: boolean;
  setIsThinkingEnabled(this: void, enabled: boolean): void;
  setSubmittedIsThinkingEnabled(this: void, enabled: boolean): void;
  submittedIsThinkingEnabled: boolean;
};

export const PromptThinkingContext = createContext<PromptThinkingContextValue>({
  get isThinkingEnabled(): never {
    throw new Error("PromptThinkingContext not provided");
  },
  setIsThinkingEnabled(): never {
    throw new Error("PromptThinkingContext not provided");
  },
  setSubmittedIsThinkingEnabled(): never {
    throw new Error("PromptThinkingContext not provided");
  },
  get submittedIsThinkingEnabled(): never {
    throw new Error("PromptThinkingContext not provided");
  },
});
