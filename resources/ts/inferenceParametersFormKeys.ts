import type { InferenceParameters } from "@intentee/paddler-client/schemas/InferenceParameters";

export type InferenceParametersBooleanKeys = {
  [TKey in keyof InferenceParameters]: TKey extends string
    ? InferenceParameters[TKey] extends boolean
      ? TKey
      : never
    : never;
}[keyof InferenceParameters];

export type InferenceParametersNumberKeys = {
  [TKey in keyof InferenceParameters]: TKey extends string
    ? InferenceParameters[TKey] extends number
      ? TKey
      : never
    : never;
}[keyof InferenceParameters];
