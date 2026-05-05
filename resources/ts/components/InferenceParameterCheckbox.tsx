import React, { useCallback, useContext } from "react";

import { InferenceParametersContext } from "../contexts/InferenceParametersContext";
import { type InferenceParametersBooleanKeys } from "../inferenceParametersFormKeys";
import {
  inferenceParameterInput,
  inferenceParameterInput__checkbox,
  inferenceParameterInput__label,
} from "./inferenceParameterInput.module.css";

// eslint-disable-next-line @typescript-eslint/no-unnecessary-type-parameters
export function InferenceParameterCheckbox<TKey extends InferenceParametersBooleanKeys>({
  description,
  name,
}: {
  description: string;
  name: TKey;
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
