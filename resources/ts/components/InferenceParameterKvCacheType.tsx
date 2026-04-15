import React, { useCallback, useContext, type ChangeEvent } from "react";

import { InferenceParametersContext } from "../contexts/InferenceParametersContext";
import { kvCacheTypes } from "../schemas/InferenceParameters";
import {
  inferenceParameterInput,
  inferenceParameterInput__label,
  inferenceParameterInput__select,
} from "./inferenceParameterInput.module.css";

const name = "kv_cache_type";

function isKvCacheType(value: string): value is (typeof kvCacheTypes)[number] {
  return kvCacheTypes.includes(value as (typeof kvCacheTypes)[number]);
}

export function InferenceParameterKvCacheType({
  description,
}: {
  description: string;
}) {
  const { parameters, setParameter } = useContext(InferenceParametersContext);

  const onChange = useCallback(
    function (evt: ChangeEvent<HTMLSelectElement>) {
      const option = evt.currentTarget.value;

      if (!isKvCacheType(option)) {
        throw new Error(`Invalid KV cache type: ${option}`);
      }

      setParameter(name, option);
    },
    [setParameter],
  );

  return (
    <label className={inferenceParameterInput}>
      <abbr className={inferenceParameterInput__label} title={description}>
        {name}
      </abbr>
      <div className={inferenceParameterInput__select}>
        <select name={name} value={parameters[name]} onChange={onChange}>
          {kvCacheTypes.map(function (option: string) {
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
