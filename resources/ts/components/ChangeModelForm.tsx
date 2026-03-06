import React, {
  useCallback,
  useContext,
  useMemo,
  type FormEvent,
  type InputEvent,
} from "react";
import { useLocation } from "wouter";

import { ChatTemplateContext } from "../contexts/ChatTemplateContext";
import { InferenceParametersContext } from "../contexts/InferenceParametersContext";
import { PaddlerConfigurationContext } from "../contexts/PaddlerConfigurationContext";
import { useAgentDesiredModelUrl } from "../hooks/useAgentDesiredModelUrl";
import { type BalancerDesiredState } from "../schemas/BalancerDesiredState";
import { ChatTemplateBehavior } from "./ChatTemplateBehavior";
import { InferenceParameterCheckbox } from "./InferenceParameterCheckbox";
import { InferenceParameterInput } from "./InferenceParameterInput";
import { InferenceParameterPoolingType } from "./InferenceParameterPoolingType";

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

export function ChangeModelForm({
  defaultBaseModelUri,
  defaultMultimodalProjectionUri,
}: {
  defaultBaseModelUri: null | string;
  defaultMultimodalProjectionUri: null | string;
}) {
  const [, navigate] = useLocation();
  const { chatTemplateOverride, useChatTemplateOverride } = useContext(ChatTemplateContext);
  const { parameters } = useContext(InferenceParametersContext);
  const { managementAddr } = useContext(PaddlerConfigurationContext);
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

  const onBaseModelUriInput = useCallback(
    function (evt: InputEvent<HTMLInputElement>) {
      setBaseModelUri(evt.currentTarget.value);
    },
    [setBaseModelUri],
  );

  const onMultimodalProjectionUriInput = useCallback(
    function (evt: InputEvent<HTMLInputElement>) {
      setMultimodalProjectionModelUri(evt.currentTarget.value);
    },
    [setMultimodalProjectionModelUri],
  );

  const balancerDesiredState: null | BalancerDesiredState = useMemo(
    function () {
      if (!baseModelAgentDesiredModelState.ok || !multimodalProjecttionAgentDesiredModelState.ok) {
        return null;
      }

      const desiredState: BalancerDesiredState = Object.freeze({
        chat_template_override: chatTemplateOverride,
        inference_parameters: parameters,
        model: baseModelAgentDesiredModelState.agentDesiredModel,
        multimodal_projection: multimodalProjecttionAgentDesiredModelState.agentDesiredModel,
        use_chat_template_override: useChatTemplateOverride,
      });

      return desiredState;
    },
    [
      baseModelAgentDesiredModelState,
      chatTemplateOverride,
      multimodalProjecttionAgentDesiredModelState,
      parameters,
      useChatTemplateOverride,
    ],
  );

  const onSubmit = useCallback(
    function (evt: FormEvent<HTMLFormElement>) {
      evt.preventDefault();

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
              Each agent will download the model individually and cache it locally.
            </p>
            <p>
              For example, you can use the following URL to download the
              Qwen-3.5 0.8B model:
            </p>
            <code>
              https://huggingface.co/unsloth/Qwen3.5-0.8B-GGUF/blob/main/Qwen3.5-0.8B-Q4_K_M.gguf
            </code>
            <p>
              To enable multimodal features, you also need to provide multimodal projection weights relevant to the base model
              (usually, model authors name them similarly to mmproj-*.gguf)
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
            <div className={changeModelForm__formLabel__title}>Base Model URI</div>
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
          <label className={changeModelForm__formLabel}>
            <div className={changeModelForm__formLabel__title}>Multimodal Projection URI (optional)</div>
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
          <fieldset className={changeModelForm__parameters}>
            <legend>Inference Parameters</legend>
            <details className={changeModelForm__details}>
              <summary>What are these parameters?</summary>
              <p>
                These parameters control how the model behaves during inference.
                They can affect the quality, speed, and memory usage of the
                model.
              </p>
              <p>
                They are usually model-specific and are usually provided by the
                model authors, although Paddler provides some reasonable
                defaults.
              </p>
              <p>
                Experimenting with these settings is worth exploring to optimize
                performance for your specific needs.
              </p>
            </details>
            <InferenceParameterInput
              description="Batch Size (higher = more memory usage, lower = less inference speed)"
              name="batch_n_tokens"
            />
            <InferenceParameterInput
              description="Context Size (higher = longer chat history, lower = less memory usage)"
              name="context_size"
            />
            <InferenceParameterInput
              description="Max simultaneous sequences per embedding batch (higher = more throughput, more memory)"
              name="embedding_n_seq_max"
            />
            <InferenceParameterInput
              description="Minimum token probability to consider for selection"
              name="min_p"
            />
            <InferenceParameterInput
              description="Frequency Penalty"
              name="penalty_frequency"
            />
            <InferenceParameterInput
              description="Number of last tokens to consider for penalty (-1 = entire context, 0 = disabled)"
              name="penalty_last_n"
            />
            <InferenceParameterInput
              description="Presence Penalty"
              name="penalty_presence"
            />
            <InferenceParameterInput
              description="Repeated Token Penalty"
              name="penalty_repeat"
            />
            <InferenceParameterInput
              description="Temperature"
              name="temperature"
            />
            <InferenceParameterInput
              description="Number of tokens to consider for selection"
              name="top_k"
            />
            <InferenceParameterInput
              description="Probability threshold for selecting tokens"
              name="top_p"
            />
            <InferenceParameterCheckbox
              description="You need embeddings for stuff like semantic search, RAG, and more"
              name="enable_embeddings"
            />
            <InferenceParameterPoolingType
              description="How to combine token embeddings"
              disabled={!parameters.enable_embeddings}
            />
          </fieldset>
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
