import React from "react";

import { type AgentIssueSeverity } from "../schemas/AgentIssue";
import {
  notificationCount___error,
  notificationCount___warning,
} from "./NotificationCount.module.css";

const classNameBySeverity: Record<AgentIssueSeverity, string> = {
  Error: notificationCount___error,
  Warning: notificationCount___warning,
};

export function NotificationCount({
  count,
  severity,
}: {
  count: number;
  severity: AgentIssueSeverity;
}) {
  return <span className={classNameBySeverity[severity]}>{count}</span>;
}
