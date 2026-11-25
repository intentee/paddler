import React, { useCallback, useContext } from "react";

import { type PromptMedia } from "../PromptMedia";
import { PromptMediaContext } from "../contexts/PromptMediaContext";

import {
  conversationPromptInputMediaPreview,
  conversationPromptInputMediaPreview__audio,
  conversationPromptInputMediaPreview__image,
  conversationPromptInputMediaPreview__remove,
} from "./ConversationPromptInputMediaPreview.module.css";

export function ConversationPromptInputMediaPreview({
  media,
}: {
  media: PromptMedia;
}) {
  const { setCurrentMedia } = useContext(PromptMediaContext);

  const onRemove = useCallback(
    function () {
      setCurrentMedia(null);
    },
    [setCurrentMedia],
  );

  return (
    <div className={conversationPromptInputMediaPreview}>
      {"audio" === media.kind ? (
        <audio
          className={conversationPromptInputMediaPreview__audio}
          controls
          src={media.dataUri}
        />
      ) : (
        <img
          className={conversationPromptInputMediaPreview__image}
          src={media.dataUri}
          alt="Attached"
        />
      )}
      <button
        className={conversationPromptInputMediaPreview__remove}
        type="button"
        onClick={onRemove}
      >
        &times;
      </button>
    </div>
  );
}
