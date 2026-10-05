import React, { useCallback, useContext, type MouseEvent } from "react";

import iconChat from "../../icons/chat.svg";
import { ModelMetadataContext } from "../contexts/ModelMetadataContext";
import { modelChatTemplatePreviewButton } from "./ModelChatTemplatePreviewButton.module.css";

export function ModelChatTemplatePreviewButton({
  metadataKey,
}: {
  metadataKey: string;
}) {
  const { focusedMetadataParameter, metadata, setFocusedMetadataParameter } =
    useContext(ModelMetadataContext);

  const onClick = useCallback(
    function (evt: MouseEvent<HTMLButtonElement>) {
      evt.preventDefault();

      const metadataValue = metadata[metadataKey];

      if ("string" !== typeof metadataValue) {
        throw new Error("Expected metadata value to be a string");
      }

      setFocusedMetadataParameter({
        metadataKey,
        metadataValue,
      });
    },
    [setFocusedMetadataParameter, metadata, metadataKey],
  );

  return (
    <button
      className={modelChatTemplatePreviewButton}
      disabled={focusedMetadataParameter?.metadataKey === metadataKey}
      onClick={onClick}
    >
      <img src={iconChat} alt="Chat Template" />
      Chat Template
    </button>
  );
}
