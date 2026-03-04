import clsx from "clsx";
import React from "react";

import { type Agent } from "../schemas/Agent";
import { type AgentIssue } from "../schemas/AgentIssue";
import { AgentIssuesPreviewButton } from "./AgentIssuesPreviewButton";
import { AgentListAgentStatus } from "./AgentListAgentStatus";
import { ModelChatTemplateOverridePreviewButton } from "./ModelChatTemplateOverridePreviewButton";
import { ModelMetadataPreviewButton } from "./ModelMetadataPreviewButton";

import iconDownload from "../../icons/download.svg";
import {
  agentList,
  agentList__agent,
  agentList__agentHasErrors,
  agentList__agent__download,
  agentList__agent__issues,
  agentList__agent__issues__list,
  agentList__agent__metadata,
  agentList__agent__model,
  agentList__agent__model__name,
  agentList__agent__name,
  agentList__agent__status,
} from "./AgentList.module.css";

function hasErrorSeverity(issues: Array<AgentIssue>): boolean {
  return issues.some(function (issue) {
    return issue.severity === "Error";
  });
}

function splitBySeverity(issues: Array<AgentIssue>): {
  errors: Array<AgentIssue>;
  warnings: Array<AgentIssue>;
} {
  const errors: Array<AgentIssue> = [];
  const warnings: Array<AgentIssue> = [];

  for (const issue of issues) {
    if (issue.severity === "Error") {
      errors.push(issue);
    } else {
      warnings.push(issue);
    }
  }

  return { errors, warnings };
}

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
          download_current,
          download_filename,
          download_total,
          id,
          issues,
          model_path,
          name,
          uses_chat_template_override,
        } = agent;

        return (
          <div
            className={clsx(agentList__agent, {
              [agentList__agentHasErrors]:
                issues.length > 0 && hasErrorSeverity(issues),
            })}
            key={id}
          >
            <div className={agentList__agent__issues}>
              <div className={agentList__agent__name}>{name}</div>
              {issues.length > 0 ? (
                <div className={agentList__agent__issues__list}>
                  {(function () {
                    const { errors, warnings } = splitBySeverity(issues);

                    return (
                      <>
                        {errors.length > 0 && (
                          <AgentIssuesPreviewButton
                            agentName={name}
                            issues={errors}
                          />
                        )}
                        {warnings.length > 0 && (
                          <AgentIssuesPreviewButton
                            agentName={name}
                            issues={warnings}
                          />
                        )}
                      </>
                    );
                  })()}
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
            {download_total > 0 ? (
              <div className={agentList__agent__download}>
                <progress max={download_total} value={download_current} />
                <abbr title={`Downloading: ${download_filename}`}>
                  <img src={iconDownload} alt="Download" />
                </abbr>
              </div>
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
