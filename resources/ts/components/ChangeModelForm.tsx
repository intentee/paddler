import React, { useCallback, useContext, type FormEvent } from "react";
import { useLocation } from "wouter";

import { type AgentDesiredModel } from "@intentee/paddler-client/schemas/AgentDesiredModel";
import { inferenceModes } from "@intentee/paddler-client/schemas/InferenceMode";
import { BalancerDesiredStateContext } from "../contexts/BalancerDesiredStateContext";
import { PaddlerConfigurationContext } from "../contexts/PaddlerConfigurationContext";
import { ChatTemplateBehavior } from "./ChatTemplateBehavior";
import { EmbeddingParametersFields } from "./EmbeddingParametersFields";
import { ModelRuntimeParametersFields } from "./ModelRuntimeParametersFields";
import { ModelUriInput } from "./ModelUriInput";
import { ParameterSelect } from "./ParameterSelect";
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
  changeModelForm__main,
  changeModelForm__parameters,
  changeModelForm__submitButton,
} from "./ChangeModelForm.module.css";

export function ChangeModelForm() {
  const [, navigate] = useLocation();
  const {
    balancerDesiredState: editedDesiredState,
    setBalancerDesiredState: setEditedDesiredState,
  } = useContext(BalancerDesiredStateContext);
  const { inference_mode: inferenceMode } = editedDesiredState;
  const { managementAddr } = useContext(PaddlerConfigurationContext);

  const onBaseModel = useCallback(
    function (model: AgentDesiredModel) {
      setEditedDesiredState({ ...editedDesiredState, model });
    },
    [editedDesiredState, setEditedDesiredState],
  );

  const onMultimodalProjection = useCallback(
    function (projection: AgentDesiredModel) {
      setEditedDesiredState({
        ...editedDesiredState,
        text_generation: {
          ...editedDesiredState.text_generation,
          multimodal: {
            ...editedDesiredState.text_generation.multimodal,
            projection,
          },
        },
      });
    },
    [editedDesiredState, setEditedDesiredState],
  );

  const onPointerHead = useCallback(
    function (pointer_head: AgentDesiredModel) {
      setEditedDesiredState({
        ...editedDesiredState,
        decision: { ...editedDesiredState.decision, pointer_head },
      });
    },
    [editedDesiredState, setEditedDesiredState],
  );

  const onSubmit = useCallback(
    function (event: FormEvent<HTMLFormElement>) {
      event.preventDefault();

      fetch(`//${managementAddr}/api/v1/balancer_desired_state`, {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(editedDesiredState),
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
    [editedDesiredState, managementAddr, navigate],
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
          <fieldset className={changeModelForm__parameters}>
            <legend>Inference Mode</legend>
            <ParameterSelect
              description="What the cluster serves; switching it reloads every agent, and requests of the other modes are refused"
              name="inference_mode"
              onValue={function (selectedInferenceMode) {
                setEditedDesiredState({
                  ...editedDesiredState,
                  inference_mode: selectedInferenceMode,
                });
              }}
              options={inferenceModes}
              value={inferenceMode}
            />
          </fieldset>
          <label className={changeModelForm__formLabel}>
            <div className={changeModelForm__formLabel__title}>
              Base Model URI
            </div>
            <ModelUriInput
              model={editedDesiredState.model}
              name="model_uri"
              onModel={onBaseModel}
              required
            />
          </label>
          {inferenceMode === "Decision" && (
            <label className={changeModelForm__formLabel}>
              <div className={changeModelForm__formLabel__title}>
                Pointer Head URI
              </div>
              <ModelUriInput
                model={editedDesiredState.decision.pointer_head}
                name="pointer_head_uri"
                onModel={onPointerHead}
                required
              />
            </label>
          )}
          {inferenceMode === "TextGeneration" && (
            <>
              <label className={changeModelForm__formLabel}>
                <div className={changeModelForm__formLabel__title}>
                  Multimodal Projection URI (optional)
                </div>
                <ModelUriInput
                  model={
                    editedDesiredState.text_generation.multimodal.projection
                  }
                  name="multimodal_projection_uri"
                  onModel={onMultimodalProjection}
                  required={false}
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
