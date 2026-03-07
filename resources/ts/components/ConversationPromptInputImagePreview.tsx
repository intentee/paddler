import React, { useCallback, useContext } from "react";

import { PromptImageContext } from "../contexts/PromptImageContext";

import {
  conversationPromptInputImagePreview,
  conversationPromptInputImagePreview__image,
  conversationPromptInputImagePreview__remove,
} from "./ConversationPromptInputImagePreview.module.css";

export function ConversationPromptInputImagePreview({
  imageDataUri,
}: {
  imageDataUri: string;
}) {
  const { setCurrentImageDataUri } = useContext(PromptImageContext);

  const onRemove = useCallback(
    function () {
      setCurrentImageDataUri(null);
    },
    [setCurrentImageDataUri],
  );
  return (
    <div className={conversationPromptInputImagePreview}>
      <img
        className={conversationPromptInputImagePreview__image}
        src={imageDataUri}
        alt="Attached"
      />
      <button
        className={conversationPromptInputImagePreview__remove}
        type="button"
        onClick={onRemove}
      >
        &times;
      </button>
    </div>
  );
}
