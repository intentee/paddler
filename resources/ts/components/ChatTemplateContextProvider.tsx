import React, { useContext, useMemo, type ReactNode } from "react";

import { type BalancerTextGenerationSettings } from "@intentee/paddler-client/schemas/BalancerTextGenerationSettings";
import { type ChatTemplate } from "@intentee/paddler-client/schemas/ChatTemplate";
import { BalancerDesiredStateContext } from "../contexts/BalancerDesiredStateContext";
import {
  ChatTemplateContext,
  type ChatTemplateContextValue,
} from "../contexts/ChatTemplateContext";
import { textGenerationSettingsOf } from "../textGenerationSettingsOf";

export function ChatTemplateContextProvider({
  children,
}: {
  children: ReactNode;
}) {
  const { balancerDesiredState, setBalancerDesiredState } = useContext(
    BalancerDesiredStateContext,
  );

  const value = useMemo<ChatTemplateContextValue>(
    function () {
      const textGenerationSettings =
        textGenerationSettingsOf(balancerDesiredState);

      function setTextGenerationSettings(
        changedSettings: Partial<BalancerTextGenerationSettings>,
      ) {
        setBalancerDesiredState({
          ...balancerDesiredState,
          inference_settings: {
            TextGeneration: {
              ...textGenerationSettings,
              ...changedSettings,
            },
          },
        });
      }

      return Object.freeze({
        chatTemplateOverride: textGenerationSettings.chat_template_override,
        setChatTemplateOverride(chatTemplateOverride: null | ChatTemplate) {
          setTextGenerationSettings({
            chat_template_override: chatTemplateOverride,
          });
        },
        setChatTemplateOverrideContent(content: string) {
          setTextGenerationSettings({ chat_template_override: { content } });
        },
        setUseChatTemplateOverride(useChatTemplateOverride: boolean) {
          setTextGenerationSettings({
            use_chat_template_override: useChatTemplateOverride,
          });
        },
        useChatTemplateOverride:
          textGenerationSettings.use_chat_template_override,
      });
    },
    [balancerDesiredState, setBalancerDesiredState],
  );

  return (
    <ChatTemplateContext.Provider value={value}>
      {children}
    </ChatTemplateContext.Provider>
  );
}
