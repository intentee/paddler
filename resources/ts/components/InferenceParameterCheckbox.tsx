import React, { useCallback, useContext } from "react";

import { InferenceParametersContext } from "../contexts/InferenceParametersContext";
import { type InferenceParametersBooleanKeys } from "../inferenceParametersFormKeys";
import {
  inferenceParameterInput,
  inferenceParameterInput__checkbox,
  inferenceParameterInput__label,
} from "./inferenceParameterInput.module.css";

export function InferenceParameterCheckbox({
  description,
  name,
}: {
  description: string;
  name: InferenceParametersBooleanKeys;
}) {
  const { parameters, setParameter } = useContext(InferenceParametersContext);

  const onChange = useCallback(
    function () {
      setParameter(name, !parameters[name]);
    },
    [name, parameters, setParameter],
  );

  return (
    <label className={inferenceParameterInput}>
      <abbr className={inferenceParameterInput__label} title={description}>
        {name}
      </abbr>
      <div className={inferenceParameterInput__checkbox}>
        <input
          checked={parameters[name]}
          name={name}
          onChange={onChange}
          type="checkbox"
        />
      </div>
    </label>
  );
}
