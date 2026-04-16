import React, { useCallback, useContext, type ChangeEvent } from "react";

import { InferenceParametersContext } from "../contexts/InferenceParametersContext";
import { cacheDtypes } from "../schemas/InferenceParameters";
import {
  inferenceParameterInput,
  inferenceParameterInput__label,
  inferenceParameterInput__select,
} from "./inferenceParameterInput.module.css";

function isCacheDtype(value: string): value is (typeof cacheDtypes)[number] {
  return cacheDtypes.includes(value as (typeof cacheDtypes)[number]);
}

export function InferenceParameterCacheDtype({
  description,
  name,
}: {
  description: string;
  name: "k_cache_dtype" | "v_cache_dtype";
}) {
  const { parameters, setParameter } = useContext(InferenceParametersContext);

  const onChange = useCallback(
    function (evt: ChangeEvent<HTMLSelectElement>) {
      const option = evt.currentTarget.value;

      if (!isCacheDtype(option)) {
        throw new Error(`Invalid cache dtype: ${option}`);
      }

      setParameter(name, option);
    },
    [name, setParameter],
  );

  return (
    <label className={inferenceParameterInput}>
      <abbr className={inferenceParameterInput__label} title={description}>
        {name}
      </abbr>
      <div className={inferenceParameterInput__select}>
        <select name={name} value={parameters[name]} onChange={onChange}>
          {cacheDtypes.map(function (option: string) {
            return (
              <option key={option} value={option}>
                {option}
              </option>
            );
          })}
        </select>
      </div>
    </label>
  );
}
