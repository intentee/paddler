import clsx from "clsx";
import React from "react";

import { type Agent } from "@intentee/paddler-client/schemas/Agent";
import { type ModelDownloadStatus } from "@intentee/paddler-client/schemas/ModelDownloadStatus";
import { AgentIssuesPreviewButton } from "./AgentIssuesPreviewButton";
import { AgentListAgentStatus } from "./AgentListAgentStatus";
import { ModelChatTemplateOverridePreviewButton } from "./ModelChatTemplateOverridePreviewButton";
import { ModelMetadataPreviewButton } from "./ModelMetadataPreviewButton";

import iconDownload from "../../icons/download.svg";
import {
  agentList,
  agentList__agent,
  agentList__agentHasIssues,
  agentList__agent__download,
  agentList__agent__issues,
  agentList__agent__issues__list,
  agentList__agent__metadata,
  agentList__agent__model,
  agentList__agent__model__name,
  agentList__agent__name,
  agentList__agent__status,
} from "./AgentList.module.css";

function displayLastPathPart(path: string | null): string {
  if (!path) {
    return "";
  }

  const parts = path.split("/");
  const last = parts.pop();

  if (!last) {
    return "";
  }

  return last;
}

function AgentModelDownload({
  downloadStatus,
}: {
  downloadStatus: Exclude<ModelDownloadStatus, "NotDownloading">;
}) {
  const { model_path, progress } =
    "Downloading" in downloadStatus
      ? {
          model_path: downloadStatus.Downloading.model_path,
          progress: (
            <progress
              max={downloadStatus.Downloading.total_bytes}
              value={downloadStatus.Downloading.downloaded_bytes}
            />
          ),
        }
      : {
          model_path: downloadStatus.DownloadingWithUnknownSize.model_path,
          progress: <progress />,
        };

  return (
    <div className={agentList__agent__download}>
      {progress}
      <abbr title={`Downloading: ${model_path}`}>
        <img src={iconDownload} alt="Download" />
      </abbr>
    </div>
  );
}

export function AgentList({
  agents,
  managementAddr,
}: {
  agents: Array<Agent>;
  managementAddr: string;
}) {
  return (
    <div className={agentList}>
      {agents.map(function (agent: Agent) {
        const {
          id,
          name,
          status: {
            download_status,
            issues,
            model_path,
            uses_chat_template_override,
          },
        } = agent;

        return (
          <div
            className={clsx(agentList__agent, {
              [agentList__agentHasIssues]: issues.length > 0,
            })}
            key={id}
          >
            <div className={agentList__agent__issues}>
              <div className={agentList__agent__name}>{name}</div>
              {issues.length > 0 ? (
                <div className={agentList__agent__issues__list}>
                  <AgentIssuesPreviewButton agentName={name} issues={issues} />
                </div>
              ) : (
                <div className={agentList__agent__issues__list}>
                  👍 <i>OK</i>
                </div>
              )}
            </div>
            <div className={agentList__agent__metadata}>
              <ModelMetadataPreviewButton
                agent={agent}
                managementAddr={managementAddr}
              />
              {uses_chat_template_override && (
                <ModelChatTemplateOverridePreviewButton
                  agent={agent}
                  managementAddr={managementAddr}
                />
              )}
            </div>
            {"NotDownloading" !== download_status ? (
              <AgentModelDownload downloadStatus={download_status} />
            ) : (
              <div className={agentList__agent__model}>
                {"string" === typeof model_path ? (
                  <abbr
                    className={agentList__agent__model__name}
                    title={model_path}
                  >
                    {displayLastPathPart(model_path)}
                  </abbr>
                ) : (
                  <i className={agentList__agent__model__name}>
                    No model loaded
                  </i>
                )}
              </div>
            )}
            <div className={agentList__agent__status}>
              <AgentListAgentStatus agent={agent} />
            </div>
          </div>
        );
      })}
    </div>
  );
}
