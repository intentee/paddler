import React, { useCallback, useContext } from "react";

import { type SamplingParameters } from "@intentee/paddler-client/schemas/SamplingParameters";
import { BalancerDesiredStateContext } from "../contexts/BalancerDesiredStateContext";
import { textGenerationSettingsOf } from "../textGenerationSettingsOf";
import { ParameterNumberInput } from "./ParameterNumberInput";

const samplingParameterDescriptions: ReadonlyArray<{
  description: string;
  name: keyof SamplingParameters;
}> = [
  {
    description: "Minimum token probability to consider for selection",
    name: "min_p",
  },
  { description: "Frequency Penalty", name: "penalty_frequency" },
  {
    description: "Number of last tokens to consider for penalty (0 = disabled)",
    name: "penalty_last_n",
  },
  { description: "Presence Penalty", name: "penalty_presence" },
  { description: "Repeated Token Penalty", name: "penalty_repeat" },
  { description: "Temperature", name: "temperature" },
  {
    description: "Number of tokens to consider for selection",
    name: "top_k",
  },
  {
    description: "Probability threshold for selecting tokens",
    name: "top_p",
  },
];

export function TextGenerationSettingsFields() {
  const { balancerDesiredState, setBalancerDesiredState } = useContext(
    BalancerDesiredStateContext,
  );
  const textGenerationSettings = textGenerationSettingsOf(balancerDesiredState);

  const setSamplingParameter = useCallback(
    function (name: keyof SamplingParameters, value: number) {
      setBalancerDesiredState({
        ...balancerDesiredState,
        inference_settings: {
          TextGeneration: {
            ...textGenerationSettings,
            sampling_parameters: {
              ...textGenerationSettings.sampling_parameters,
              [name]: value,
            },
          },
        },
      });
    },
    [balancerDesiredState, setBalancerDesiredState, textGenerationSettings],
  );

  const setImageResizeToFit = useCallback(
    function (imageResizeToFit: number) {
      setBalancerDesiredState({
        ...balancerDesiredState,
        inference_settings: {
          TextGeneration: {
            ...textGenerationSettings,
            multimodal: {
              ...textGenerationSettings.multimodal,
              image_resize_to_fit: imageResizeToFit,
            },
          },
        },
      });
    },
    [balancerDesiredState, setBalancerDesiredState, textGenerationSettings],
  );

  return (
    <>
      <ParameterNumberInput
        description="Max image dimension in pixels before resizing"
        name="image_resize_to_fit"
        onValue={setImageResizeToFit}
        value={textGenerationSettings.multimodal.image_resize_to_fit}
      />
      {samplingParameterDescriptions.map(function ({ description, name }) {
        return (
          <ParameterNumberInput
            description={description}
            key={name}
            name={name}
            onValue={function (value) {
              setSamplingParameter(name, value);
            }}
            value={textGenerationSettings.sampling_parameters[name]}
          />
        );
      })}
    </>
  );
}
