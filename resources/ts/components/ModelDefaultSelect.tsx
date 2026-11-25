import React, { useCallback, type ChangeEvent } from "react";

import {
  inferenceParameterInput,
  inferenceParameterInput__label,
  inferenceParameterInput__select,
} from "./inferenceParameterInput.module.css";

const MODEL_DEFAULT_OPTION = "";

export function ModelDefaultSelect<TOption extends string>({
  description,
  name,
  onValue,
  options,
  value,
}: {
  description: string;
  name: string;
  onValue(this: void, value: TOption | null): void;
  options: ReadonlyArray<TOption>;
  value: TOption | null;
}) {
  const onChange = useCallback(
    function (event: ChangeEvent<HTMLSelectElement>) {
      const selectedValue = event.currentTarget.value;

      if (selectedValue === MODEL_DEFAULT_OPTION) {
        onValue(null);

        return;
      }

      const selectedOption = options.find(function (option) {
        return option === selectedValue;
      });

      if (selectedOption === undefined) {
        throw new Error(`Invalid ${name}: ${selectedValue}`);
      }

      onValue(selectedOption);
    },
    [name, onValue, options],
  );

  return (
    <label className={inferenceParameterInput}>
      <abbr className={inferenceParameterInput__label} title={description}>
        {name}
      </abbr>
      <div className={inferenceParameterInput__select}>
        <select
          name={name}
          value={value ?? MODEL_DEFAULT_OPTION}
          onChange={onChange}
        >
          <option value={MODEL_DEFAULT_OPTION}>model default</option>
          {options.map(function (option) {
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
