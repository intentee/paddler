import { createContext } from "react";

import { type ChatTemplate } from "@intentee/paddler-client/schemas/ChatTemplate";

export type ChatTemplateContextValue = {
  chatTemplateOverride: null | ChatTemplate;
  setChatTemplateOverride(
    this: void,
    chatTemplateOverride: null | ChatTemplate,
  ): void;
  setChatTemplateOverrideContent(this: void, content: string): void;
  setUseChatTemplateOverride(
    this: void,
    useChatTemplateOverride: boolean,
  ): void;
  useChatTemplateOverride: boolean;
};

export const ChatTemplateContext = createContext<ChatTemplateContextValue>({
  get chatTemplateOverride(): never {
    throw new Error("ChatTemplateContext not provided");
  },
  get useChatTemplateOverride(): never {
    throw new Error("ChatTemplateContext not provided");
  },
  setChatTemplateOverride(): void {
    throw new Error("ChatTemplateContext not provided");
  },
  setChatTemplateOverrideContent(): void {
    throw new Error("ChatTemplateContext not provided");
  },
  setUseChatTemplateOverride(): void {
    throw new Error("ChatTemplateContext not provided");
  },
});
