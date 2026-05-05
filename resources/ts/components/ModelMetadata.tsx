import React, { useContext } from "react";

import { ModelMetadataContext } from "../contexts/ModelMetadataContext";
import { type Agent } from "@intentee/paddler-client/schemas/Agent";
import { ModalWindow } from "./ModalWindow";
import { ModelChatTemplatePreviewButton } from "./ModelChatTemplatePreviewButton";
import { ModelMetadataFocusedParameter } from "./ModelMetadataFocusedParameter";

import {
  modelMetadata,
  modelMetadata__parameter,
  modelMetadata__parameter__title,
  modelMetadata__parameter__value,
  modelMetadata__templateOverrideNote,
} from "./ModelMetadata.module.css";

export function ModelMetadata({
  agent: { name, uses_chat_template_override },
  onClose,
}: {
  agent: Agent;
  onClose(this: void): void;
}) {
  const { focusedMetadataParameter, metadata } =
    useContext(ModelMetadataContext);

  return (
    <ModalWindow
      onClose={onClose}
      title={`${name} / Metadata${focusedMetadataParameter ? ` / ${focusedMetadataParameter.metadataKey}` : ""}`}
    >
      {focusedMetadataParameter ? (
        <ModelMetadataFocusedParameter
          focusedMetadataParameter={focusedMetadataParameter}
        />
      ) : (
        <div className={modelMetadata}>
          {Object.entries(metadata).map(function ([
            metadataKey,
            metadataValue,
          ]) {
            return (
              <div className={modelMetadata__parameter} key={metadataKey}>
                <div className={modelMetadata__parameter__title}>
                  {metadataKey}:
                </div>
                <div className={modelMetadata__parameter__value}>
                  {"tokenizer.chat_template" === metadataKey ? (
                    <>
                      <ModelChatTemplatePreviewButton
                        metadataKey={metadataKey}
                      />

                      {uses_chat_template_override && (
                        <p>
                          <i className={modelMetadata__templateOverrideNote}>
                            <strong>Note:</strong> Model does not use this chat
                            template at the moment, because you provided a
                            custom chat template.
                          </i>
                        </p>
                      )}
                    </>
                  ) : (
                    metadataValue
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </ModalWindow>
  );
}
