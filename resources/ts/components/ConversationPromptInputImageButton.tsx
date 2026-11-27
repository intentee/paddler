import React, { useCallback, useContext } from "react";

import { PromptMediaContext } from "../contexts/PromptMediaContext";
import { readFileAsDataUri } from "../readFileAsDataUri";
import { ConversationPromptInputMediaButton } from "./ConversationPromptInputMediaButton";

import iconImage from "../../icons/image.svg";

export function ConversationPromptInputImageButton() {
  const { setCurrentMedia } = useContext(PromptMediaContext);

  const onFile = useCallback(
    function (imageFile: File) {
      void readFileAsDataUri(imageFile).then(function (dataUri) {
        setCurrentMedia({ dataUri, kind: "image" });
      });
    },
    [setCurrentMedia],
  );

  return (
    <ConversationPromptInputMediaButton
      accept="image/*"
      iconAlt="Add image"
      iconSrc={iconImage}
      onFile={onFile}
    />
  );
}
