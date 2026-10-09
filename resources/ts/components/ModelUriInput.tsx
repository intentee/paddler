import React, { useCallback, type InputEvent } from "react";

import { type AgentDesiredModel } from "@intentee/paddler-client/schemas/AgentDesiredModel";

import { changeModelForm__input } from "./ChangeModelForm.module.css";

export function ModelUriInput({
  model,
  name,
  onModel,
  required,
}: {
  model: AgentDesiredModel;
  name: string;
  onModel(this: void, model: AgentDesiredModel): void;
  required: boolean;
}) {
  const onInput = useCallback(
    function (event: InputEvent<HTMLInputElement>) {
      const uri = event.currentTarget.value;

      onModel(uri === "" ? "None" : { Uri: uri });
    },
    [onModel],
  );

  return (
    <input
      className={changeModelForm__input}
      name={name}
      onInput={onInput}
      placeholder="https://huggingface.co/..."
      required={required}
      type="text"
      value={model === "None" ? "" : model.Uri}
    />
  );
}
