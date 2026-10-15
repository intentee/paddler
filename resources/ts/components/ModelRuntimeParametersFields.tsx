import React, { useCallback, useContext } from "react";

import {
  cacheDtypes,
  type ModelRuntimeParameters,
} from "@intentee/paddler-client/schemas/ModelRuntimeParameters";
import { BalancerDesiredStateContext } from "../contexts/BalancerDesiredStateContext";
import { ParameterNumberInput } from "./ParameterNumberInput";
import { ParameterSelect } from "./ParameterSelect";

export function ModelRuntimeParametersFields() {
  const { balancerDesiredState, setBalancerDesiredState } = useContext(
    BalancerDesiredStateContext,
  );
  const modelRuntimeParameters = balancerDesiredState.model_runtime_parameters;

  const setParameter = useCallback(
    function <TKey extends keyof ModelRuntimeParameters>(
      name: TKey,
      value: ModelRuntimeParameters[TKey],
    ) {
      setBalancerDesiredState({
        ...balancerDesiredState,
        model_runtime_parameters: {
          ...balancerDesiredState.model_runtime_parameters,
          [name]: value,
        },
      });
    },
    [balancerDesiredState, setBalancerDesiredState],
  );

  return (
    <>
      <ParameterNumberInput
        description="Batch Size (higher = more memory usage, lower = less inference speed)"
        name="n_batch"
        onValue={function (value) {
          setParameter("n_batch", value);
        }}
        value={modelRuntimeParameters.n_batch}
      />
      <ParameterNumberInput
        description="Context Size (higher = longer inputs, lower = less memory usage)"
        name="context_size"
        onValue={function (value) {
          setParameter("context_size", value);
        }}
        value={modelRuntimeParameters.context_size}
      />
      <ParameterNumberInput
        description="Number of model layers to offload to GPU (0 = CPU only; -1 = all layers)"
        name="n_gpu_layers"
        onValue={function (value) {
          setParameter("n_gpu_layers", value);
        }}
        value={modelRuntimeParameters.n_gpu_layers}
      />
      <ParameterSelect
        description="Datatype for K cache tensors"
        name="k_cache_dtype"
        onValue={function (value) {
          setParameter("k_cache_dtype", value);
        }}
        options={cacheDtypes}
        value={modelRuntimeParameters.k_cache_dtype}
      />
      <ParameterSelect
        description="Datatype for V cache tensors"
        name="v_cache_dtype"
        onValue={function (value) {
          setParameter("v_cache_dtype", value);
        }}
        options={cacheDtypes}
        value={modelRuntimeParameters.v_cache_dtype}
      />
    </>
  );
}
