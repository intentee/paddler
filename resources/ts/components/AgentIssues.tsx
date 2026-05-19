import React from "react";
import { Link } from "wouter";

import { type AgentIssue } from "@intentee/paddler-client/schemas/AgentIssue";

import { agentIssues, agentIssues__issue } from "./AgentIssues.module.css";

export function AgentIssues({ issues }: { issues: Array<AgentIssue> }) {
  return (
    <ul className={agentIssues}>
      {issues.map(function (issue, index) {
        if ("ChatTemplateDoesNotCompile" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Chat template does not compile: "
                {issue.ChatTemplateDoesNotCompile.error}"
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
                <code>{issue.ChatTemplateDoesNotCompile.template_content}</code>
              </pre>
            </li>
          );
        }

        if ("HuggingFaceCannotAcquireLock" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                HuggingFace cannot acquire lock:{" "}
                {issue.HuggingFaceCannotAcquireLock.model_path.model_path}
              </strong>
              <strong>Lock path:</strong>{" "}
              <pre>
                <code>{issue.HuggingFaceCannotAcquireLock.lock_path}</code>
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

        if ("HuggingFaceModelDoesNotExist" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                HuggingFace model does not exist:{" "}
                {issue.HuggingFaceModelDoesNotExist.model_path}
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

        if ("HuggingFacePermissions" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                You do not have enough permissions to download model from
                HuggingFace: {issue.HuggingFacePermissions.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will retry to download the model in case this might be a
                temporary issue.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                <Link href="/model">You need to check the model URL.</Link>
                There is a chance that you made a typo in the organization name.
                In that case HuggingFace reports 401 error instead of 404.
              </p>
            </li>
          );
        }

        if ("ModelCannotBeLoaded" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Model cannot be loaded: {issue.ModelCannotBeLoaded.model_path}
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

        if ("ModelFileDoesNotExist" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Model file does not exist:{" "}
                {issue.ModelFileDoesNotExist.model_path}
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

        if ("MultimodalProjectionCannotBeLoaded" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Multimodal projection cannot be loaded:{" "}
                {issue.MultimodalProjectionCannotBeLoaded.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will continue to run, but it will not reattempt to load
                the model.
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

        if ("SlotCannotStart" in issue) {
          const { error, slot_index } = issue.SlotCannotStart;

          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Unable to start slot {slot_index}: {error}
              </strong>
            </li>
          );
        }

        if ("UnableToFindChatTemplate" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Unable to find chat template:{" "}
                {issue.UnableToFindChatTemplate.model_path}
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

        if ("DownloadUrlIsMalformed" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Download URL is malformed:{" "}
                {issue.DownloadUrlIsMalformed.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will keep re-checking, but the same malformed URL
                will keep failing the same way.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                <Link href="/model">
                  Edit the model URL on the model configuration page
                </Link>{" "}
                to a valid http or https URL.
              </p>
            </li>
          );
        }

        if ("ModelDoesNotExistAtUrl" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Model does not exist at URL:{" "}
                {issue.ModelDoesNotExistAtUrl.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will keep re-checking, but the same 404 will keep
                firing until the remote server publishes the file at that URL.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                Check the URL — the file may have moved or been removed.{" "}
                <Link href="/model">Update the URL</Link> or replace it with one
                that resolves.
              </p>
            </li>
          );
        }

        if ("DownloadServerDeniedAccess" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Download server denied access:{" "}
                {issue.DownloadServerDeniedAccess.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will keep re-checking; if the server starts
                accepting the request, the next attempt will succeed.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                Confirm the URL is correct and reachable without auth. If it's
                a private model, switch to a URL that doesn't require
                credentials, or use the HuggingFace integration instead.
              </p>
            </li>
          );
        }

        if ("DownloadServerErrored" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Download server returned an error status:{" "}
                {issue.DownloadServerErrored.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will keep re-checking. If the server starts
                answering normally, the next attempt will succeed.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                The remote server is reachable but returning a 5xx response.
                Check the server's status page or logs if you control it;
                otherwise wait — overload and maintenance windows usually clear
                on the server's end.
              </p>
            </li>
          );
        }

        if ("DownloadInterrupted" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Download was interrupted:{" "}
                {issue.DownloadInterrupted.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will keep re-checking. The next attempt resumes
                from the bytes already on disk if the server supports Range
                requests; otherwise it starts fresh.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                Often transient — check network stability and whether the
                remote server is being restarted or rate-limiting. No action
                needed if it clears on its own.
              </p>
            </li>
          );
        }

        if ("DownloadServerIsUnreachable" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Download server is unreachable:{" "}
                {issue.DownloadServerIsUnreachable.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will keep re-checking; if the network comes back,
                the next attempt will succeed.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                Check the agent's internet connection, firewall rules, and the
                remote server's status.
              </p>
            </li>
          );
        }

        if ("ServerRejectedRangeRequest" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Remote server rejected a partial-download resume:{" "}
                {issue.ServerRejectedRangeRequest.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler has already discarded the stale partial file. The next
                re-check will start fresh.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                Usually no action needed; the next tick recovers automatically.
                If it persists, the remote file changed — replace the URL if
                you need the new content under a stable path.
              </p>
            </li>
          );
        }

        if ("CacheDirectoryIsNotWritable" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Cache directory is not writable:{" "}
                {issue.CacheDirectoryIsNotWritable.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will keep re-checking; the moment write permission
                is restored, the next attempt will succeed.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                Grant write permission to the cache directory (
                <code>$XDG_CACHE_HOME/paddler</code> on Linux/macOS,{" "}
                <code>%LOCALAPPDATA%\paddler</code> on Windows), or set{" "}
                <code>PADDLER_CACHE_DIR</code> to a writable location.
              </p>
            </li>
          );
        }

        if ("CacheStorageIsFull" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Cache storage is full while downloading:{" "}
                {issue.CacheStorageIsFull.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will keep re-checking; the moment space is
                available, the next attempt will succeed.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>Free space on the disk that hosts the cache directory.</p>
            </li>
          );
        }

        if ("ModelCacheIsCorrupted" in issue) {
          return (
            <li className={agentIssues__issue} key={index}>
              <strong>
                Model cache is corrupted:{" "}
                {issue.ModelCacheIsCorrupted.model_path}
              </strong>
              <strong>What will Paddler do?</strong>{" "}
              <p>
                Paddler will keep re-checking; the cache will be rebuilt
                on the next attempt.
              </p>
              <strong>What can you do?</strong>{" "}
              <p>
                If the issue persists, manually clear the{" "}
                <code>downloaded-models</code> subdirectory of the cache and
                let Paddler rebuild it.
              </p>
            </li>
          );
        }

        const _exhaustive: never = issue;
        return _exhaustive;
      })}
    </ul>
  );
}
