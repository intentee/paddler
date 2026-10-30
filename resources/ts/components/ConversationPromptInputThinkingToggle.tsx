import clsx from "clsx";
import React, { useCallback, useContext } from "react";

import { PromptThinkingContext } from "../contexts/PromptThinkingContext";

import { conversationPromptInput__button } from "./ConversationPromptInput.module.css";
import { conversationPromptInputThinkingToggleActive } from "./ConversationPromptInputThinkingToggle.module.css";

import iconLightOff from "../../icons/light_off.svg";
import iconLightbulb from "../../icons/lightbulb.svg";

export function ConversationPromptInputThinkingToggle() {
  const { isThinkingEnabled, setIsThinkingEnabled } = useContext(
    PromptThinkingContext,
  );

  const onToggle = useCallback(
    function () {
      setIsThinkingEnabled(!isThinkingEnabled);
    },
    [isThinkingEnabled, setIsThinkingEnabled],
  );

  return (
    <button
      className={clsx(conversationPromptInput__button, {
        [conversationPromptInputThinkingToggleActive]: isThinkingEnabled,
      })}
      type="button"
      onClick={onToggle}
    >
      <img
        src={isThinkingEnabled ? iconLightbulb : iconLightOff}
        alt="Toggle thinking"
      />
    </button>
  );
}
