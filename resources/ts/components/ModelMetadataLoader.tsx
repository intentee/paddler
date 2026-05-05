import React from "react";

import { useModelMetadata } from "../hooks/useModelMetadata";
import { matchFetchJsonState } from "../matchFetchJsonState";
import { type Agent } from "@intentee/paddler-client/schemas/Agent";
import { ModalWindow } from "./ModalWindow";
import { ModelMetadata } from "./ModelMetadata";
import { ModelMetadataContextProvider } from "./ModelMetadataContextProvider";

import iconHourglass from "../../icons/hourglass.svg";
import {
  modalWindowLoader,
  modalWindowLoader__spinner,
} from "./modalWindowLoader.module.css";

export function ModelMetadataLoader({
  agent,
  managementAddr,
  onClose,
}: {
  agent: Agent;
  managementAddr: string;
  onClose(this: void): void;
}) {
  const { id, name } = agent;
  const loadingState = useModelMetadata({
    agentId: id,
    managementAddr,
  });

  return (
    <div className={modalWindowLoader}>
      {matchFetchJsonState(loadingState, {
        empty() {
          return (
            <ModalWindow onClose={onClose} title={`${name} / No Model`}>
              <span>No model loaded</span>
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
                <span>Loading model metadata...</span>
              </div>
            </ModalWindow>
          );
        },
        ok({ response }) {
          if (!response) {
            return (
              <ModalWindow onClose={onClose} title={`${name}`}>
                <span>No model loaded</span>
              </ModalWindow>
            );
          }

          return (
            <ModelMetadataContextProvider metadata={response.metadata}>
              <ModelMetadata agent={agent} onClose={onClose} />
            </ModelMetadataContextProvider>
          );
        },
      })}
    </div>
  );
}
