import React, {
  useCallback,
  useContext,
  type InputEvent,
  type SubmitEvent,
} from "react";

import {
  conversationPromptInput,
  conversationPromptInput__attachments,
  conversationPromptInput__button,
  conversationPromptInput__controls,
  conversationPromptInput__controls__track,
  conversationPromptInput__textarea,
} from "./ConversationPromptInput.module.css";

import iconArrowUpward from "../../icons/arrow_upward.svg";

import { type ConversationFeatures } from "../ConversationFeatures";
import { type PromptMedia } from "../PromptMedia";
import { PromptContext } from "../contexts/PromptContext";
import { PromptMediaContext } from "../contexts/PromptMediaContext";
import { PromptSystemMessageContext } from "../contexts/PromptSystemMessageContext";
import { PromptThinkingContext } from "../contexts/PromptThinkingContext";
import { ConversationPromptInputAudioButton } from "./ConversationPromptInputAudioButton";
import { ConversationPromptInputImageButton } from "./ConversationPromptInputImageButton";
import { ConversationPromptInputMediaPreview } from "./ConversationPromptInputMediaPreview";
import { ConversationPromptInputThinkingToggle } from "./ConversationPromptInputThinkingToggle";

function isMediaAccepted(
  media: PromptMedia,
  {
    acceptsAudio,
    acceptsImages,
  }: Pick<ConversationFeatures, "acceptsAudio" | "acceptsImages">,
): boolean {
  return "audio" === media.kind ? acceptsAudio : acceptsImages;
}

export function ConversationPromptInput({
  conversationFeatures: {
    acceptsAudio,
    acceptsImages,
    supportsSystemRole,
    supportsThinking,
  },
}: {
  conversationFeatures: ConversationFeatures;
}) {
  const {
    currentPrompt,
    isCurrentPromptEmpty,
    setCurrentPrompt,
    setSubmittedPrompt,
  } = useContext(PromptContext);

  const { currentMedia, setCurrentMedia, setSubmittedMedia } =
    useContext(PromptMediaContext);
  const acceptedMedia =
    currentMedia !== null &&
    isMediaAccepted(currentMedia, { acceptsAudio, acceptsImages })
      ? currentMedia
      : null;
  const { isThinkingEnabled, setSubmittedIsThinkingEnabled } = useContext(
    PromptThinkingContext,
  );
  const { setSubmittedIsSystemMessageIncluded } = useContext(
    PromptSystemMessageContext,
  );

  const onSubmit = useCallback(
    function (event: SubmitEvent<HTMLFormElement>) {
      event.preventDefault();

      if (currentPrompt.trim() === "") {
        setSubmittedPrompt(null);
        setSubmittedMedia(null);
      } else {
        setSubmittedPrompt(currentPrompt);
        setSubmittedMedia(acceptedMedia);
        setSubmittedIsThinkingEnabled(supportsThinking && isThinkingEnabled);
        setSubmittedIsSystemMessageIncluded(supportsSystemRole);
        setCurrentMedia(null);
      }
    },
    [
      acceptedMedia,
      currentPrompt,
      isThinkingEnabled,
      setCurrentMedia,
      setSubmittedMedia,
      setSubmittedIsSystemMessageIncluded,
      setSubmittedIsThinkingEnabled,
      setSubmittedPrompt,
      supportsSystemRole,
      supportsThinking,
    ],
  );

  const onTextareaInput = useCallback(
    function (event: InputEvent<HTMLInputElement>) {
      setCurrentPrompt(event.currentTarget.value);
    },
    [setCurrentPrompt],
  );

  return (
    <form className={conversationPromptInput} onSubmit={onSubmit}>
      {acceptedMedia !== null && (
        <div className={conversationPromptInput__attachments}>
          <ConversationPromptInputMediaPreview media={acceptedMedia} />
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
          {supportsThinking && <ConversationPromptInputThinkingToggle />}
        </div>
        <div className={conversationPromptInput__controls__track}>
          {acceptsAudio && <ConversationPromptInputAudioButton />}
          {acceptsImages && <ConversationPromptInputImageButton />}
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
