import { createContext } from "react";

export type ChatTemplateContextValue = {
  chatTemplateOverrideContent: string;
  setChatTemplateOverrideContent(this: void, content: string): void;
  setUseChatTemplateOverride(
    this: void,
    useChatTemplateOverride: boolean,
  ): void;
  useChatTemplateOverride: boolean;
};

export const ChatTemplateContext = createContext<ChatTemplateContextValue>({
  get chatTemplateOverrideContent(): never {
    throw new Error("ChatTemplateContext not provided");
  },
  get useChatTemplateOverride(): never {
    throw new Error("ChatTemplateContext not provided");
  },
  setChatTemplateOverrideContent(): void {
    throw new Error("ChatTemplateContext not provided");
  },
  setUseChatTemplateOverride(): void {
    throw new Error("ChatTemplateContext not provided");
  },
});
