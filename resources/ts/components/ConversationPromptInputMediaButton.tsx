import React, { useCallback, useRef, type ChangeEvent } from "react";

import { conversationPromptInput__button } from "./ConversationPromptInput.module.css";
import { conversationPromptInputMediaButton__fileInput } from "./ConversationPromptInputMediaButton.module.css";

export function ConversationPromptInputMediaButton({
  accept,
  iconAlt,
  iconSrc,
  onFile,
}: {
  accept: string;
  iconAlt: string;
  iconSrc: string;
  onFile(this: void, file: File): void;
}) {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const onButtonClick = useCallback(function () {
    fileInputRef.current?.click();
  }, []);

  const onFileSelected = useCallback(
    function (event: ChangeEvent<HTMLInputElement>) {
      const selectedFile = event.currentTarget.files?.[0];

      if (!selectedFile) {
        return;
      }

      onFile(selectedFile);

      event.currentTarget.value = "";
    },
    [onFile],
  );

  return (
    <>
      <input
        accept={accept}
        className={conversationPromptInputMediaButton__fileInput}
        ref={fileInputRef}
        type="file"
        onChange={onFileSelected}
      />
      <button
        className={conversationPromptInput__button}
        type="button"
        onClick={onButtonClick}
      >
        <img src={iconSrc} alt={iconAlt} />
      </button>
    </>
  );
}
