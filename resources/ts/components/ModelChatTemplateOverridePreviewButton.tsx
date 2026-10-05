import React, { useCallback, useState, type MouseEvent } from "react";

import { type Agent } from "@intentee/paddler-client/schemas/Agent";
import { ChatTemplateOverrideLoader } from "./ChatTemplateOverrideLoader";

import { modelChatTemplateOverridePreviewButton } from "./ModelChatTemplateOverridePreviewButton.module.css";

export function ModelChatTemplateOverridePreviewButton({
  agent,
  managementAddr,
}: {
  agent: Agent;
  managementAddr: string;
}) {
  const [isPreviewVisible, setIsPreviewVisible] = useState(false);

  const onClick = useCallback(
    function (evt: MouseEvent<HTMLButtonElement>) {
      evt.preventDefault();

      setIsPreviewVisible(true);
    },
    [setIsPreviewVisible],
  );

  const onClose = useCallback(
    function () {
      setIsPreviewVisible(false);
    },
    [setIsPreviewVisible],
  );

  return (
    <>
      <button
        className={modelChatTemplateOverridePreviewButton}
        onClick={onClick}
      >
        Custom chat template
      </button>
      {isPreviewVisible && (
        <ChatTemplateOverrideLoader
          agent={agent}
          managementAddr={managementAddr}
          onClose={onClose}
        />
      )}
    </>
  );
}
