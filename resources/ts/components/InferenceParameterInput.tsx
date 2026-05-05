import React, { useCallback, useContext, type FormEvent } from "react";

import { InferenceParametersContext } from "../contexts/InferenceParametersContext";
import { type InferenceParameters } from "@intentee/paddler-client/schemas/InferenceParameters";
import { type InferenceParametersNumberKeys } from "../inferenceParametersFormKeys";
import {
  inferenceParameterInput,
  inferenceParameterInput__input,
  inferenceParameterInput__label,
} from "./inferenceParameterInput.module.css";

// eslint-disable-next-line @typescript-eslint/no-unnecessary-type-parameters
export function InferenceParameterInput<TKey extends InferenceParametersNumberKeys>({
  description,
  name,
}: {
  description: string;
  name: TKey;
}) {
  const { parameters, setParameter } = useContext(InferenceParametersContext);

  const onInput = useCallback(
    function (event: FormEvent<HTMLInputElement>) {
      event.preventDefault();

      setParameter(
        name,
        parseFloat(event.currentTarget.value) as InferenceParameters[TKey],
      );
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
