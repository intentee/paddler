import React, { useCallback, type InputEvent } from "react";

import {
  inferenceParameterInput,
  inferenceParameterInput__input,
  inferenceParameterInput__label,
} from "./inferenceParameterInput.module.css";

export function ModelDefaultNumberInput({
  description,
  name,
  onValue,
  value,
}: {
  description: string;
  name: string;
  onValue(this: void, value: number | null): void;
  value: number | null;
}) {
  const onInput = useCallback(
    function (event: InputEvent<HTMLInputElement>) {
      event.preventDefault();

      const typedValue = event.currentTarget.value;

      onValue(typedValue === "" ? null : parseFloat(typedValue));
    },
    [onValue],
  );

  return (
    <label className={inferenceParameterInput}>
      <abbr className={inferenceParameterInput__label} title={description}>
        {name}
      </abbr>
      <input
        className={inferenceParameterInput__input}
        name={name}
        onInput={onInput}
        placeholder="model default"
        type="number"
        value={value ?? ""}
      />
    </label>
  );
}
