import React, {
  useCallback,
  useContext,
  useMemo,
  type FormEvent,
  type InputEvent,
} from "react";
import { useLocation } from "wouter";

import { type AgentDesiredModel } from "@intentee/paddler-client/schemas/AgentDesiredModel";
import { type BalancerDesiredState } from "@intentee/paddler-client/schemas/BalancerDesiredState";
import { type BalancerInferenceSettings } from "@intentee/paddler-client/schemas/BalancerInferenceSettings";
import { BalancerDesiredStateContext } from "../contexts/BalancerDesiredStateContext";
import { PaddlerConfigurationContext } from "../contexts/PaddlerConfigurationContext";
import { useAgentDesiredModelUrl } from "../hooks/useAgentDesiredModelUrl";
import { ChatTemplateBehavior } from "./ChatTemplateBehavior";
import { EmbeddingParametersFields } from "./EmbeddingParametersFields";
import { ModelRuntimeParametersFields } from "./ModelRuntimeParametersFields";
import { TextGenerationSettingsFields } from "./TextGenerationSettingsFields";

import {
  changeModelForm,
  changeModelForm__asideInfo,
  changeModelForm__chatTemplate,
  changeModelForm__details,
  changeModelForm__form,
  changeModelForm__formControls,
  changeModelForm__formLabel,
  changeModelForm__formLabel__title,
  changeModelForm__input,
  changeModelForm__main,
  changeModelForm__parameters,
  changeModelForm__submitButton,
} from "./ChangeModelForm.module.css";

function withMultimodalProjection(
  inferenceSettings: BalancerInferenceSettings,
  projection: AgentDesiredModel,
): BalancerInferenceSettings {
  if (!("TextGeneration" in inferenceSettings)) {
    return inferenceSettings;
  }

  return {
    TextGeneration: {
      ...inferenceSettings.TextGeneration,
      multimodal: {
        ...inferenceSettings.TextGeneration.multimodal,
        projection,
      },
    },
  };
}

function withPointerHead(
  inferenceSettings: BalancerInferenceSettings,
  pointerHead: AgentDesiredModel,
): BalancerInferenceSettings {
  if (!("Decision" in inferenceSettings)) {
    return inferenceSettings;
  }

  return {
    Decision: {
      ...inferenceSettings.Decision,
      pointer_head: pointerHead,
    },
  };
}

export function ChangeModelForm({
  defaultBaseModelUri,
  defaultMultimodalProjectionUri,
  defaultPointerHeadUri,
}: {
  defaultBaseModelUri: null | string;
  defaultMultimodalProjectionUri: null | string;
  defaultPointerHeadUri: null | string;
}) {
  const [, navigate] = useLocation();
  const { balancerDesiredState: editedDesiredState } = useContext(
    BalancerDesiredStateContext,
  );
  const { inferenceMode, managementAddr } = useContext(
    PaddlerConfigurationContext,
  );
  const {
    agentDesiredModelState: baseModelAgentDesiredModelState,
    modelUri: baseModelUri,
    setModelUri: setBaseModelUri,
  } = useAgentDesiredModelUrl({
    defaultModelUri: defaultBaseModelUri,
  });
  const {
    agentDesiredModelState: multimodalProjecttionAgentDesiredModelState,
    modelUri: multimodalProjectionModelUri,
    setModelUri: setMultimodalProjectionModelUri,
  } = useAgentDesiredModelUrl({
    defaultModelUri: defaultMultimodalProjectionUri,
  });
  const {
    agentDesiredModelState: pointerHeadAgentDesiredModelState,
    modelUri: pointerHeadModelUri,
    setModelUri: setPointerHeadModelUri,
  } = useAgentDesiredModelUrl({
    defaultModelUri: defaultPointerHeadUri,
  });

  const onBaseModelUriInput = useCallback(
    function (event: InputEvent<HTMLInputElement>) {
      setBaseModelUri(event.currentTarget.value);
    },
    [setBaseModelUri],
  );

  const onMultimodalProjectionUriInput = useCallback(
    function (event: InputEvent<HTMLInputElement>) {
      setMultimodalProjectionModelUri(event.currentTarget.value);
    },
    [setMultimodalProjectionModelUri],
  );

  const onPointerHeadUriInput = useCallback(
    function (event: InputEvent<HTMLInputElement>) {
      setPointerHeadModelUri(event.currentTarget.value);
    },
    [setPointerHeadModelUri],
  );

  const balancerDesiredState: null | BalancerDesiredState = useMemo(
    function () {
      if (
        !baseModelAgentDesiredModelState.ok ||
        !multimodalProjecttionAgentDesiredModelState.ok ||
        !pointerHeadAgentDesiredModelState.ok
      ) {
        return null;
      }

      const desiredState: BalancerDesiredState = Object.freeze({
        ...editedDesiredState,
        inference_settings: withPointerHead(
          withMultimodalProjection(
            editedDesiredState.inference_settings,
            multimodalProjecttionAgentDesiredModelState.agentDesiredModel,
          ),
          pointerHeadAgentDesiredModelState.agentDesiredModel,
        ),
        model: baseModelAgentDesiredModelState.agentDesiredModel,
      });

      return desiredState;
    },
    [
      baseModelAgentDesiredModelState,
      editedDesiredState,
      multimodalProjecttionAgentDesiredModelState,
      pointerHeadAgentDesiredModelState,
    ],
  );

  const onSubmit = useCallback(
    function (event: FormEvent<HTMLFormElement>) {
      event.preventDefault();

      if (!balancerDesiredState) {
        return;
      }

      fetch(`//${managementAddr}/api/v1/balancer_desired_state`, {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(balancerDesiredState),
      })
        .then(function (response) {
          if (response.ok) {
            navigate("/");
          } else {
            throw new Error(
              `Failed to update agent desired state: ${response.statusText}`,
            );
          }
        })
        .catch(function (error: unknown) {
          console.error("Error updating agent desired state:", error);
        });
    },
    [managementAddr, navigate, balancerDesiredState],
  );

  return (
    <div className={changeModelForm}>
      <aside className={changeModelForm__asideInfo}>
        <p>
          Paddler is based on <strong>llama.cpp</strong>, and it supports models
          in the <strong>GGUF</strong> format.
        </p>
        <p>Supported sources:</p>
        <dl>
          <dt>
            <a href="https://huggingface.co/" target="_blank">
              Hugging Face 🤗
            </a>
          </dt>
          <dd>
            <p>
              Each agent will download the model individually and cache it
              locally.
            </p>
            <p>
              For example, you can use the following URL to download the
              Qwen-3.5 0.8B model:
            </p>
            <code>
              https://huggingface.co/unsloth/Qwen3.5-0.8B-GGUF/blob/main/Qwen3.5-0.8B-Q4_K_M.gguf
            </code>
            <p>
              To enable multimodal features, you also need to provide multimodal
              projection weights relevant to the base model (usually, model
              authors name them similarly to mmproj-*.gguf)
            </p>
            <code>
              https://huggingface.co/unsloth/Qwen3.5-0.8B-GGUF/blob/main/mmproj-F16.gguf
            </code>
          </dd>
          <dt>Local File</dt>
          <dd>
            <p>File path is relative to the agent's working directory.</p>
            <p>
              If you want all the agents to use the same model, you need to
              ensure that the file is present in the same path on all agents.
            </p>
            <code>agent:///path/to/your/model.gguf</code>
          </dd>
        </dl>
      </aside>
      <main className={changeModelForm__main}>
        <form className={changeModelForm__form} onSubmit={onSubmit}>
          <label className={changeModelForm__formLabel}>
            <div className={changeModelForm__formLabel__title}>
              Base Model URI
            </div>
            <input
              className={changeModelForm__input}
              name="model_uri"
              onInput={onBaseModelUriInput}
              placeholder="https://huggingface.co/..."
              required
              type="url"
              value={String(baseModelUri)}
            />
          </label>
          {inferenceMode === "Decision" && (
            <label className={changeModelForm__formLabel}>
              <div className={changeModelForm__formLabel__title}>
                Pointer Head URI
              </div>
              <input
                className={changeModelForm__input}
                name="pointer_head_uri"
                onInput={onPointerHeadUriInput}
                placeholder="https://huggingface.co/..."
                required
                type="url"
                value={String(pointerHeadModelUri)}
              />
            </label>
          )}
          {inferenceMode === "TextGeneration" && (
            <>
              <label className={changeModelForm__formLabel}>
                <div className={changeModelForm__formLabel__title}>
                  Multimodal Projection URI (optional)
                </div>
                <input
                  className={changeModelForm__input}
                  name="multimodal_projection_uri"
                  onInput={onMultimodalProjectionUriInput}
                  placeholder="https://huggingface.co/..."
                  type="url"
                  value={String(multimodalProjectionModelUri)}
                />
              </label>
              <fieldset className={changeModelForm__chatTemplate}>
                <legend>Chat Template</legend>
                <ChatTemplateBehavior />
              </fieldset>
            </>
          )}
          <fieldset className={changeModelForm__parameters}>
            <legend>Model Parameters</legend>
            <details className={changeModelForm__details}>
              <summary>What are these parameters?</summary>
              <p>
                These parameters control how the model is loaded and how much
                memory it uses. In text generation, the context is shared
                between the slots of each agent.
              </p>
              <p>
                They are usually model-specific and are usually provided by the
                model authors, although Paddler provides some reasonable
                defaults.
              </p>
            </details>
            <ModelRuntimeParametersFields />
          </fieldset>
          {inferenceMode === "TextGeneration" && (
            <fieldset className={changeModelForm__parameters}>
              <legend>Text Generation Parameters</legend>
              <TextGenerationSettingsFields />
            </fieldset>
          )}
          {inferenceMode === "Embeddings" && (
            <fieldset className={changeModelForm__parameters}>
              <legend>Embedding Parameters</legend>
              <EmbeddingParametersFields />
            </fieldset>
          )}
          <div className={changeModelForm__formControls}>
            <button className={changeModelForm__submitButton}>
              Apply changes
            </button>
          </div>
        </form>
      </main>
    </div>
  );
}
