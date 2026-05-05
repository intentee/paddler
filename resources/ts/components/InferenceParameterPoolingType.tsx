import React, { useCallback, useContext, type ChangeEvent } from "react";

import { InferenceParametersContext } from "../contexts/InferenceParametersContext";
import { poolingTypes } from "@intentee/paddler-client/schemas/InferenceParameters";
import {
  inferenceParameterInput,
  inferenceParameterInput__disabledHint,
  inferenceParameterInput__disabledHint__content,
  inferenceParameterInput__label,
  inferenceParameterInput__select,
} from "./inferenceParameterInput.module.css";

const name = "pooling_type";

function isPoolingType(value: string): value is (typeof poolingTypes)[number] {
  return poolingTypes.includes(value as (typeof poolingTypes)[number]);
}

export function InferenceParameterPoolingType({
  description,
  disabled,
}: {
  description: string;
  disabled: boolean;
}) {
  const { parameters, setParameter } = useContext(InferenceParametersContext);

  const onChange = useCallback(
    function (evt: ChangeEvent<HTMLSelectElement>) {
      const option = evt.currentTarget.value;

      if (!isPoolingType(option)) {
        throw new Error(`Invalid pooling type: ${option}`);
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
        <select
          disabled={disabled}
          name={name}
          value={parameters[name]}
          onChange={onChange}
        >
          {poolingTypes.map(function (option: string) {
            return (
              <option key={option} value={option}>
                {option}
              </option>
            );
          })}
        </select>
        {disabled && (
          <div className={inferenceParameterInput__disabledHint}>
            <div className={inferenceParameterInput__disabledHint__content}>
              enable embeddings to turn this on
            </div>
          </div>
        )}
      </div>
    </label>
  );
}
