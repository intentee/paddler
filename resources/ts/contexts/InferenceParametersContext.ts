import { createContext } from "react";

import { type InferenceParameters } from "@intentee/paddler-client/schemas/InferenceParameters";

export type InferenceParametersContextValue = {
  parameters: InferenceParameters;
  setParameter<TKey extends keyof InferenceParameters>(
    this: void,
    name: TKey,
    value: InferenceParameters[TKey],
  ): void;
  setPartialParameters(
    this: void,
    parameters: Partial<InferenceParameters>,
  ): void;
};

export const InferenceParametersContext =
  createContext<InferenceParametersContextValue>({
    get parameters(): never {
      throw new Error("InferenceParametersContext not provided");
    },
    setParameter(): never {
      throw new Error("InferenceParametersContext not provided");
    },
    setPartialParameters(): never {
      throw new Error("InferenceParametersContext not provided");
    },
  });
