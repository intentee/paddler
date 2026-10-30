import React, { useCallback, type InputEvent } from "react";

import {
  inferenceParameterInput,
  inferenceParameterInput__input,
  inferenceParameterInput__label,
} from "./inferenceParameterInput.module.css";

export function ParameterNumberInput({
  description,
  name,
  onValue,
  value,
}: {
  description: string;
  name: string;
  onValue(this: void, value: number): void;
  value: number;
}) {
  const onInput = useCallback(
    function (event: InputEvent<HTMLInputElement>) {
      event.preventDefault();

      onValue(parseFloat(event.currentTarget.value));
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
        required
        type="number"
        value={value}
      />
    </label>
  );
}
