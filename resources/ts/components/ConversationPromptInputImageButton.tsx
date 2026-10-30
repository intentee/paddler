import React, {
  useCallback,
  useContext,
  useRef,
  type ChangeEvent,
} from "react";

import { PromptImageContext } from "../contexts/PromptImageContext";

import { conversationPromptInput__button } from "./ConversationPromptInput.module.css";
import { conversationPromptInputImageButton__fileInput } from "./ConversationPromptInputImageButton.module.css";

import iconImage from "../../icons/image.svg";

function readFileAsDataUri(imageFile: File): Promise<string> {
  return new Promise(function (resolve, reject) {
    const fileReader = new FileReader();

    fileReader.addEventListener("load", function () {
      if (typeof fileReader.result === "string") {
        resolve(fileReader.result);
      } else {
        reject(new Error("FileReader did not produce a string result"));
      }
    });

    fileReader.addEventListener("error", function () {
      reject(fileReader.error ?? new Error("FileReader encountered an error"));
    });

    fileReader.readAsDataURL(imageFile);
  });
}

export function ConversationPromptInputImageButton() {
  const { setCurrentImageDataUri } = useContext(PromptImageContext);
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

      void readFileAsDataUri(selectedFile).then(setCurrentImageDataUri);

      event.currentTarget.value = "";
    },
    [setCurrentImageDataUri],
  );

  return (
    <>
      <input
        accept="image/*"
        className={conversationPromptInputImageButton__fileInput}
        ref={fileInputRef}
        type="file"
        onChange={onFileSelected}
      />
      <button
        className={conversationPromptInput__button}
        type="button"
        onClick={onButtonClick}
      >
        <img src={iconImage} alt="Add image" />
      </button>
    </>
  );
}
