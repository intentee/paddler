import React, { useCallback, useState, type MouseEvent } from "react";

import { type AgentIssue } from "@intentee/paddler-client/schemas/AgentIssue";
import { AgentIssues } from "./AgentIssues";
import { agentIssuesPreviewButton } from "./AgentIssuesPreviewButton.module.css";
import { ModalWindow } from "./ModalWindow";
import { NotificationCount } from "./NotificationCount";

export function AgentIssuesPreviewButton({
  agentName,
  issues,
}: {
  agentName: null | string;
  issues: Array<AgentIssue>;
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
      <button className={agentIssuesPreviewButton} onClick={onClick}>
        <NotificationCount count={issues.length} />
        Issues
      </button>
      {isDetailsVisible && (
        <ModalWindow onClose={onClose} title={`${agentName} / Issues`}>
          <AgentIssues issues={issues} />
        </ModalWindow>
      )}
    </>
  );
}
