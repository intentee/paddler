import React from "react";

import { type Agent } from "@intentee/paddler-client/schemas/Agent";
import { useChatTemplateOverride } from "../hooks/useChatTemplateOverride";
import { matchFetchJsonState } from "../matchFetchJsonState";
import { CodeEditor } from "./CodeEditor";
import { ModalWindow } from "./ModalWindow";

import iconHourglass from "../../icons/hourglass.svg";
import {
  modalWindowLoader,
  modalWindowLoader__spinner,
} from "./modalWindowLoader.module.css";

export function ChatTemplateOverrideLoader({
  agent,
  managementAddr,
  onClose,
}: {
  agent: Agent;
  managementAddr: string;
  onClose(this: void): void;
}) {
  const { id, name } = agent;
  const loadingState = useChatTemplateOverride({
    agentId: id,
    managementAddr,
  });

  return (
    <div className={modalWindowLoader}>
      {matchFetchJsonState(loadingState, {
        empty() {
          return (
            <ModalWindow
              onClose={onClose}
              title={`${name} / No Chat Template Override`}
            >
              <span>No custom chat template</span>
            </ModalWindow>
          );
        },
        error({ error }) {
          return (
            <ModalWindow onClose={onClose} title={`${name} / Error`}>
              <span>Error: {error}</span>
            </ModalWindow>
          );
        },
        loading() {
          return (
            <ModalWindow onClose={onClose} title={`${name} / Loading`}>
              <div className={modalWindowLoader__spinner}>
                <img src={iconHourglass} alt="Loading..." />
                <span>Loading custom chat template...</span>
              </div>
            </ModalWindow>
          );
        },
        ok({ response }) {
          if (!response) {
            return (
              <ModalWindow onClose={onClose} title={`${name}`}>
                <span>No custom chat template loaded</span>
              </ModalWindow>
            );
          }

          return (
            <ModalWindow
              onClose={onClose}
              title={`${name} / Custom Chat Template`}
            >
              <CodeEditor editable={false} value={response.content} />
            </ModalWindow>
          );
        },
      })}
    </div>
  );
}
