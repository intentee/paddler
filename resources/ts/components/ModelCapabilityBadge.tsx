import clsx from "clsx";
import React from "react";

import { type ModelCapabilityStatus } from "../ModelCapabilityStatus";

import {
  modelCapabilityBadge,
  modelCapabilityBadgeUnsupported,
} from "./ModelCapabilityBadge.module.css";

export function ModelCapabilityBadge({
  label,
  status: { description, isSupported },
}: {
  label: string;
  status: ModelCapabilityStatus;
}) {
  return (
    <abbr
      className={clsx(modelCapabilityBadge, {
        [modelCapabilityBadgeUnsupported]: !isSupported,
      })}
      title={description}
    >
      {isSupported ? "✓" : "✗"} {label}
    </abbr>
  );
}
