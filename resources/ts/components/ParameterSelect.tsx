import React, { useCallback, type ChangeEvent } from "react";

import {
  inferenceParameterInput,
  inferenceParameterInput__label,
  inferenceParameterInput__select,
} from "./inferenceParameterInput.module.css";

export function ParameterSelect<TOption extends string>({
  description,
  name,
  onValue,
  options,
  value,
}: {
  description: string;
  name: string;
  onValue(this: void, value: TOption): void;
  options: ReadonlyArray<TOption>;
  value: TOption;
}) {
  const onChange = useCallback(
    function (event: ChangeEvent<HTMLSelectElement>) {
      const selectedOption = options.find(function (option) {
        return option === event.currentTarget.value;
      });

      if (selectedOption === undefined) {
        throw new Error(`Invalid ${name}: ${event.currentTarget.value}`);
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
        <select name={name} value={value} onChange={onChange}>
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
