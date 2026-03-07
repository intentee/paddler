import React, { useCallback, useContext, type FormEvent } from "react";

import {
  conversationPromptInput,
  conversationPromptInput__attachments,
  conversationPromptInput__button,
  conversationPromptInput__controls,
  conversationPromptInput__controls__track,
  conversationPromptInput__textarea,
} from "./ConversationPromptInput.module.css";

import iconArrowUpward from "../../icons/arrow_upward.svg";

import { PromptContext } from "../contexts/PromptContext";
import { PromptImageContext } from "../contexts/PromptImageContext";
import { PromptThinkingContext } from "../contexts/PromptThinkingContext";
import { ConversationPromptInputImageButton } from "./ConversationPromptInputImageButton";
import { ConversationPromptInputImagePreview } from "./ConversationPromptInputImagePreview";
import { ConversationPromptInputThinkingToggle } from "./ConversationPromptInputThinkingToggle";

export function ConversationPromptInput() {
  const {
    currentPrompt,
    isCurrentPromptEmpty,
    setCurrentPrompt,
    setSubmittedPrompt,
  } = useContext(PromptContext);

  const {
    currentImageDataUri,
    isImageAttached,
    setCurrentImageDataUri,
    setSubmittedImageDataUri,
  } = useContext(PromptImageContext);
  const { isThinkingEnabled, setSubmittedIsThinkingEnabled } = useContext(
    PromptThinkingContext,
  );

  const onSubmit = useCallback(
    function (event: FormEvent<HTMLFormElement>) {
      event.preventDefault();

      if (currentPrompt.trim() === "") {
        setSubmittedPrompt(null);
        setSubmittedImageDataUri(null);
      } else {
        setSubmittedPrompt(currentPrompt);
        setSubmittedImageDataUri(currentImageDataUri);
        setSubmittedIsThinkingEnabled(isThinkingEnabled);
        setCurrentImageDataUri(null);
      }
    },
    [
      currentImageDataUri,
      currentPrompt,
      isThinkingEnabled,
      setCurrentImageDataUri,
      setSubmittedImageDataUri,
      setSubmittedIsThinkingEnabled,
      setSubmittedPrompt,
    ],
  );

  const onTextareaInput = useCallback(
    function (event: FormEvent<HTMLInputElement>) {
      setCurrentPrompt(event.currentTarget.value);
    },
    [setCurrentPrompt],
  );

  return (
    <form className={conversationPromptInput} onSubmit={onSubmit}>
      {isImageAttached && (
        <div className={conversationPromptInput__attachments}>
          <ConversationPromptInputImagePreview
            imageDataUri={currentImageDataUri as string}
          />
        </div>
      )}
      <input
        autoFocus
        className={conversationPromptInput__textarea}
        placeholder="Type your prompt here..."
        value={currentPrompt}
        onInput={onTextareaInput}
      />
      <div className={conversationPromptInput__controls}>
        <div className={conversationPromptInput__controls__track}>
          <ConversationPromptInputThinkingToggle />
        </div>
        <div className={conversationPromptInput__controls__track}>
          <ConversationPromptInputImageButton />
          <button
            className={conversationPromptInput__button}
            disabled={isCurrentPromptEmpty}
          >
            <img src={iconArrowUpward} alt="Send" />
          </button>
        </div>
      </div>
    </form>
  );
}
