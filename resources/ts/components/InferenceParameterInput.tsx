import React, { useCallback, useContext, type FormEvent } from "react";

import { InferenceParametersContext } from "../contexts/InferenceParametersContext";
import { type InferenceParametersNumberKeys } from "../inferenceParametersFormKeys";
import {
  inferenceParameterInput,
  inferenceParameterInput__input,
  inferenceParameterInput__label,
} from "./inferenceParameterInput.module.css";

export function InferenceParameterInput({
  description,
  name,
}: {
  description: string;
  name: InferenceParametersNumberKeys;
}) {
  const { parameters, setParameter } = useContext(InferenceParametersContext);

  const onInput = useCallback(
    function (event: FormEvent<HTMLInputElement>) {
      event.preventDefault();

      setParameter(name, parseFloat(event.currentTarget.value));
    },
    [name, setParameter],
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
        value={parameters[name]}
      />
    </label>
  );
}
