import { createContext } from "react";

export type PromptSystemMessageContextValue = {
  setSubmittedIsSystemMessageIncluded(this: void, isIncluded: boolean): void;
  submittedIsSystemMessageIncluded: boolean;
};

export const PromptSystemMessageContext =
  createContext<PromptSystemMessageContextValue>({
    setSubmittedIsSystemMessageIncluded(): never {
      throw new Error("PromptSystemMessageContext not provided");
    },
    get submittedIsSystemMessageIncluded(): never {
      throw new Error("PromptSystemMessageContext not provided");
    },
  });
