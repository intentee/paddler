import React, { useContext, useMemo, useState, type ReactNode } from "react";

import { type ChatTemplateSource } from "@intentee/paddler-client/schemas/ChatTemplateSource";
import { BalancerDesiredStateContext } from "../contexts/BalancerDesiredStateContext";
import {
  ChatTemplateContext,
  type ChatTemplateContextValue,
} from "../contexts/ChatTemplateContext";

export function ChatTemplateContextProvider({
  children,
}: {
  children: ReactNode;
}) {
  const { balancerDesiredState, setBalancerDesiredState } = useContext(
    BalancerDesiredStateContext,
  );
  const [
    overrideContentWhileUsingTheModelTemplate,
    setOverrideContentWhileUsingTheModelTemplate,
  ] = useState("");

  const value = useMemo<ChatTemplateContextValue>(
    function () {
      const { text_generation: textGenerationSettings } = balancerDesiredState;
      const { chat_template_source: chatTemplateSource } =
        textGenerationSettings;
      const chatTemplateOverrideContent =
        chatTemplateSource === "EmbeddedInModel"
          ? overrideContentWhileUsingTheModelTemplate
          : chatTemplateSource.Override.content;

      function setChatTemplateSource(
        changedChatTemplateSource: ChatTemplateSource,
      ) {
        setBalancerDesiredState({
          ...balancerDesiredState,
          text_generation: {
            ...textGenerationSettings,
            chat_template_source: changedChatTemplateSource,
          },
        });
      }

      return Object.freeze({
        chatTemplateOverrideContent,
        setChatTemplateOverrideContent(content: string) {
          setChatTemplateSource({ Override: { content } });
        },
        setUseChatTemplateOverride(useChatTemplateOverride: boolean) {
          if (useChatTemplateOverride) {
            setChatTemplateSource({
              Override: { content: chatTemplateOverrideContent },
            });
          } else {
            setOverrideContentWhileUsingTheModelTemplate(
              chatTemplateOverrideContent,
            );
            setChatTemplateSource("EmbeddedInModel");
          }
        },
        useChatTemplateOverride: chatTemplateSource !== "EmbeddedInModel",
      });
    },
    [
      balancerDesiredState,
      overrideContentWhileUsingTheModelTemplate,
      setBalancerDesiredState,
    ],
  );

  return (
    <ChatTemplateContext.Provider value={value}>
      {children}
    </ChatTemplateContext.Provider>
  );
}
