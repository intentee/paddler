import React from "react";

import { type IssueSeverity } from "../schemas/AgentIssue";
import {
  notificationCount___error,
  notificationCount___warning,
} from "./NotificationCount.module.css";

const classNameBySeverity: Record<IssueSeverity, string> = {
  Error: notificationCount___error,
  Warning: notificationCount___warning,
};

export function NotificationCount({
  count,
  severity,
}: {
  count: number;
  severity: IssueSeverity;
}) {
  return <span className={classNameBySeverity[severity]}>{count}</span>;
}
