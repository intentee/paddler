import React from "react";
import { Link } from "wouter";

import { type AgentIssue } from "../schemas/AgentIssue";

import {
  agentIssues,
  agentIssues__issue,
  agentIssues__issue___error,
  agentIssues__issue___warning,
} from "./AgentIssues.module.css";

function severityClassName(severity: AgentIssue["severity"]): string {
  switch (severity) {
    case "Error":
      return agentIssues__issue___error;
    case "Warning":
      return agentIssues__issue___warning;
  }
}

export function AgentIssues({ issues }: { issues: Array<AgentIssue> }) {
  return (
    <ul className={agentIssues}>
      {issues.map(function (issue, index) {
        const issueType = issue.type;
        const issueClassName = `${agentIssues__issue} ${severityClassName(issue.severity)}`;

        if ("ChatTemplateDoesNotCompile" in issueType) {
          return (
            <li className={issueClassName} key={index}>
              <strong>
                Chat template does not compile: "
                {issueType.ChatTemplateDoesNotCompile.error}"
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will continue to run, but it won't reattempt to load the
                model.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                <Link href="/model">You need to fix the chat template.</Link>
              </p>
              <strong>Template in question:</strong>{" "}
              <pre>
                <code>
                  {issueType.ChatTemplateDoesNotCompile.template_content}
                </code>
              </pre>
            </li>
          );
        }

        if ("HuggingFaceCannotAcquireLock" in issueType) {
          return (
            <li className={issueClassName} key={index}>
              <strong>
                HuggingFace cannot acquire lock:{" "}
                {issueType.HuggingFaceCannotAcquireLock.model_path.model_path}
              </strong>
              <strong>Lock path:</strong>{" "}
              <pre>
                <code>{issueType.HuggingFaceCannotAcquireLock.lock_path}</code>
              </pre>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will reattempt to download the model every few seconds
                until HuggingFace can acquire the lock.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                This is likely a temporary issue. Generally it is caused by
                either running multiple agents on the same device, or by using
                HuggingFace API by more than one process at the same time.
              </p>
            </li>
          );
        }

        if ("HuggingFaceModelDoesNotExist" in issueType) {
          return (
            <li className={issueClassName} key={index}>
              <strong>
                HuggingFace model does not exist:{" "}
                {issueType.HuggingFaceModelDoesNotExist.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler got a 404 response from HuggingFace, so it will not be
                able to download the model, and it won't reattempt to download
                it.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                <Link href="/model">You need to fix the model URL.</Link>
                If you are using a custom model, ensure that the model exists on
                HuggingFace and is accessible.
              </p>
            </li>
          );
        }

        if ("ModelCannotBeLoaded" in issueType) {
          return (
            <li className={issueClassName} key={index}>
              <strong>
                Model cannot be loaded: {issueType.ModelCannotBeLoaded.model_path}
              </strong>
              <strong>What is the cause?</strong>{" "}
              <p>
                The model file exists, but the model itself is not supported by
                Paddler, the file is corrupted, or the file is not a valid
                model.
              </p>
              <p>
                Another possibility is that it was just a temporary issue, like
                system not having enough memory to load the model.
              </p>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                The issue can be temporary, so Paddler will continue to try to
                load the model every few seconds.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                Either ensure that the valid model file is available to the
                agent at a given path, or{" "}
                <Link href="/model">change the model parameters</Link> to use a
                different model file.
              </p>
              <p>Check the agent server logs for more details on the error.</p>
            </li>
          );
        }

        if ("ModelFileDoesNotExist" in issueType) {
          return (
            <li className={issueClassName} key={index}>
              <strong>
                Model file does not exist:{" "}
                {issueType.ModelFileDoesNotExist.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will continue to try to load the model file every few
                seconds until it is available.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                Either ensure that the file is available to the agent at a given
                path, or <Link href="/model">change the model parameters</Link>{" "}
                to use a different model file.
              </p>
            </li>
          );
        }

        if ("SlotCannotStart" in issueType) {
          const { error, slot_index } = issueType.SlotCannotStart;

          return (
            <li className={issueClassName} key={index}>
              <strong>
                Unable to start slot {slot_index}: {error}
              </strong>
            </li>
          );
        }

        if ("UnableToFindChatTemplate" in issueType) {
          return (
            <li className={issueClassName} key={index}>
              <strong>
                Unable to find chat template:{" "}
                {issueType.UnableToFindChatTemplate.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will not be able to use the chat template, but it will
                continue to run. It will not try to load the model again until
                you provide a chat template to use.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>You need to provide a chat template for the model to use.</p>
              <p>
                Chat templates are extremely important for the model to work
                correctly, but sometimes they are not included in the model file
                itself (especially in the older GGUF models), and need to be
                provided separately.
              </p>
              <strong>Where can I find chat templates?</strong>{" "}
              <p>
                Usually they are provided by the model author, and they can be
                found in the model's README file on HuggingFace, or in the
                model's documentation.
              </p>
            </li>
          );
        }

        return (
          <li className={issueClassName} key={index}>
            Unknown issue: {JSON.stringify(issueType)}
          </li>
        );
      })}
    </ul>
  );
}
