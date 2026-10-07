import React, { useContext } from "react";

import { type AgentDesiredModel } from "@intentee/paddler-client/schemas/AgentDesiredModel";
import { PaddlerConfigurationContext } from "../contexts/PaddlerConfigurationContext";
import { useBalancerDesiredState } from "../hooks/useBalancerDesiredState";
import { matchFetchJsonState } from "../matchFetchJsonState";
import { BalancerDesiredStateContextProvider } from "./BalancerDesiredStateContextProvider";
import { ChangeModelForm } from "./ChangeModelForm";
import { ChatTemplateContextProvider } from "./ChatTemplateContextProvider";
import { FloatingStatus } from "./FloatingStatus";

function modelSchemaToUrl(model: AgentDesiredModel): string {
  if (model === "None") {
    return "";
  }

  if ("HuggingFace" in model) {
    const { HuggingFace } = model;

    return `https://huggingface.co/${HuggingFace.repo_id}/blob/${HuggingFace.revision}/${HuggingFace.filename}`;
  }

  if ("LocalToAgent" in model) {
    return `agent://${model.LocalToAgent}`;
  }

  if ("Url" in model) {
    return model.Url.url;
  }

  throw new Error(`Unsupported model schema: ${JSON.stringify(model)}`);
}

export function ChangeModelPage() {
  const { inferenceMode, managementAddr } = useContext(
    PaddlerConfigurationContext,
  );
  const loadingState = useBalancerDesiredState({ managementAddr });

  return matchFetchJsonState(loadingState, {
    empty() {
      return (
        <FloatingStatus>Unable to pick the desired state source</FloatingStatus>
      );
    },
    error({ error }) {
      return (
        <FloatingStatus>Error loading desired state: {error}</FloatingStatus>
      );
    },
    loading() {
      return <FloatingStatus>Loading desired state...</FloatingStatus>;
    },
    ok({ response: balancerDesiredState }) {
      const changeModelForm = (
        <ChangeModelForm
          defaultBaseModelUri={modelSchemaToUrl(balancerDesiredState.model)}
          defaultMultimodalProjectionUri={
            "TextGeneration" in balancerDesiredState.inference_settings
              ? modelSchemaToUrl(
                  balancerDesiredState.inference_settings.TextGeneration
                    .multimodal.projection,
                )
              : ""
          }
          defaultPointerHeadUri={
            "Decision" in balancerDesiredState.inference_settings
              ? modelSchemaToUrl(
                  balancerDesiredState.inference_settings.Decision.pointer_head,
                )
              : ""
          }
        />
      );

      return (
        <BalancerDesiredStateContextProvider
          defaultBalancerDesiredState={balancerDesiredState}
        >
          {inferenceMode === "TextGeneration" ? (
            <ChatTemplateContextProvider>
              {changeModelForm}
            </ChatTemplateContextProvider>
          ) : (
            changeModelForm
          )}
        </BalancerDesiredStateContextProvider>
      );
    },
  });
}
