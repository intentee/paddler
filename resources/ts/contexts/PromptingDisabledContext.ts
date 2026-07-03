import { createContext } from "react";

export type PromptingDisabledContextValue = {
  isPromptingDisabled: boolean;
};

export const PromptingDisabledContext =
  createContext<PromptingDisabledContextValue>({
    get isPromptingDisabled(): never {
      throw new Error("PromptingDisabledContext not provided");
    },
  });
