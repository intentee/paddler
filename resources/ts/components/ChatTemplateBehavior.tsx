import React, { useCallback, useContext, type ChangeEvent } from "react";

import { ChatTemplateContext } from "../contexts/ChatTemplateContext";
import { ChatTemplateEditButton } from "./ChatTemplateEditButton";

import {
  chatTemplateBehavior,
  chatTemplateBehavior__description,
  chatTemplateBehavior__option,
  chatTemplateBehavior__optionList,
  chatTemplateBehavior__radio,
} from "./ChatTemplateBehavior.module.css";

const USE_MODEL_TEMPLATE = "use_model_template";
const USE_MY_TEMPLATE = "use_my_template";

export function ChatTemplateBehavior() {
  const { setUseChatTemplateOverride, useChatTemplateOverride } =
    useContext(ChatTemplateContext);

  const onRadioChange = useCallback(
    function (evt: ChangeEvent<HTMLInputElement>) {
      switch (evt.target.value) {
        case USE_MODEL_TEMPLATE:
          setUseChatTemplateOverride(false);
          break;
        case USE_MY_TEMPLATE:
          setUseChatTemplateOverride(true);
          break;
        default:
          throw new Error("Unexpected value for chat template behavior");
      }
    },
    [setUseChatTemplateOverride],
  );

  return (
    <div className={chatTemplateBehavior}>
      <p>How should Paddler obtain the chat template?</p>
      <div className={chatTemplateBehavior__optionList}>
        <label className={chatTemplateBehavior__option}>
          <div className={chatTemplateBehavior__radio}>
            <input
              checked={!useChatTemplateOverride}
              name="chat_template_behavior"
              onChange={onRadioChange}
              required
              type="radio"
              value={USE_MODEL_TEMPLATE}
            />
          </div>
          <div className={chatTemplateBehavior__description}>
            <p>
              <strong>
                Use the chat template provided by the model (recommended)
              </strong>
            </p>
            <p>
              Most models support this, but not all. If the model does not have
              a chat template, it will fail to load.
            </p>
          </div>
        </label>
        <label className={chatTemplateBehavior__option}>
          <div className={chatTemplateBehavior__radio}>
            <input
              checked={useChatTemplateOverride}
              name="chat_template_behavior"
              onChange={onRadioChange}
              required
              type="radio"
              value={USE_MY_TEMPLATE}
            />
          </div>
          <div className={chatTemplateBehavior__description}>
            <p>
              <strong>Use my chat template</strong>
            </p>
            <p>
              Only use this option if you know the model does not come with a
              chat template, or you want to modify it to suit your needs.
            </p>
            <ChatTemplateEditButton />
          </div>
        </label>
      </div>
    </div>
  );
}
