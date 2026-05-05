import React, { useCallback, useState, type MouseEvent } from "react";

import { type Agent } from "@intentee/paddler-client/schemas/Agent";
import { ModelMetadataLoader } from "./ModelMetadataLoader";

import { modelMetadataPreviewButton } from "./ModelMetadataPreviewButton.module.css";

export function ModelMetadataPreviewButton({
  agent,
  managementAddr,
}: {
  agent: Agent;
  managementAddr: string;
}) {
  const [isDetailsVisible, setIsDetailsVisible] = useState(false);

  const onClick = useCallback(
    function (evt: MouseEvent<HTMLButtonElement>) {
      evt.preventDefault();

      setIsDetailsVisible(true);
    },
    [setIsDetailsVisible],
  );

  const onClose = useCallback(
    function () {
      setIsDetailsVisible(false);
    },
    [setIsDetailsVisible],
  );

  return (
    <>
      <button className={modelMetadataPreviewButton} onClick={onClick}>
        Metadata
      </button>
      {isDetailsVisible && (
        <ModelMetadataLoader
          agent={agent}
          managementAddr={managementAddr}
          onClose={onClose}
        />
      )}
    </>
  );
}
