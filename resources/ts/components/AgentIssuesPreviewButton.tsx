import React, { useCallback, useState, type MouseEvent } from "react";

import { type AgentIssue } from "../schemas/AgentIssue";
import { AgentIssues } from "./AgentIssues";
import {
  agentIssuesPreviewButton___error,
  agentIssuesPreviewButton___warning,
} from "./AgentIssuesPreviewButton.module.css";
import { ModalWindow } from "./ModalWindow";
import { NotificationCount } from "./NotificationCount";

function hasErrorSeverity(issues: Array<AgentIssue>): boolean {
  return issues.some(function (issue) {
    return issue.severity === "Error";
  });
}

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

  const severity = hasErrorSeverity(issues) ? "Error" : "Warning";

  const buttonClassName =
    severity === "Error"
      ? agentIssuesPreviewButton___error
      : agentIssuesPreviewButton___warning;

  return (
    <>
      <button className={buttonClassName} onClick={onClick}>
        <NotificationCount count={issues.length} severity={severity} />
        {issues.length > 1 ? `${severity}s` : severity}
      </button>
      {isDetailsVisible && (
        <ModalWindow onClose={onClose} title={`${agentName} / Issues`}>
          <AgentIssues issues={issues} />
        </ModalWindow>
      )}
    </>
  );
}
