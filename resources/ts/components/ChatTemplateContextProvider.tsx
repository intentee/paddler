import React, { useCallback, useMemo, useState, type ReactNode } from "react";

import {
  ChatTemplateContext,
  type ChatTemplateContextValue,
} from "../contexts/ChatTemplateContext";
import { type ChatTemplate } from "@intentee/paddler-client/schemas/ChatTemplate";

export function ChatTemplateContextProvider({
  children,
  defaultChatTemplateOverride,
  defaultUseChatTemplateOverride,
}: {
  children: ReactNode;
  defaultChatTemplateOverride: null | ChatTemplate;
  defaultUseChatTemplateOverride: boolean;
}) {
  const [chatTemplateOverride, setChatTemplateOverride] =
    useState<null | ChatTemplate>(defaultChatTemplateOverride);
  const [useChatTemplateOverride, setUseChatTemplateOverride] =
    useState<boolean>(defaultUseChatTemplateOverride);

  const setChatTemplateOverrideContent = useCallback(
    function (content: string) {
      setChatTemplateOverride(function () {
        return {
          content,
        };
      });
    },
    [setChatTemplateOverride],
  );

  const value = useMemo<ChatTemplateContextValue>(
    function () {
      return Object.freeze({
        chatTemplateOverride,
        setChatTemplateOverride,
        setChatTemplateOverrideContent,
        setUseChatTemplateOverride,
        useChatTemplateOverride,
      });
    },
    [
      chatTemplateOverride,
      setChatTemplateOverride,
      setChatTemplateOverrideContent,
      setUseChatTemplateOverride,
      useChatTemplateOverride,
    ],
  );

  return (
    <ChatTemplateContext.Provider value={value}>
      {children}
    </ChatTemplateContext.Provider>
  );
}
