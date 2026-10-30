import React, { useCallback, useContext } from "react";

import {
  poolingTypes,
  type EmbeddingParameters,
} from "@intentee/paddler-client/schemas/EmbeddingParameters";
import { BalancerDesiredStateContext } from "../contexts/BalancerDesiredStateContext";
import { ParameterNumberInput } from "./ParameterNumberInput";
import { ParameterSelect } from "./ParameterSelect";

export function EmbeddingParametersFields() {
  const { balancerDesiredState, setBalancerDesiredState } = useContext(
    BalancerDesiredStateContext,
  );
  const { embeddings: embeddingParameters } = balancerDesiredState;

  const setParameter = useCallback(
    function <TKey extends keyof EmbeddingParameters>(
      name: TKey,
      value: EmbeddingParameters[TKey],
    ) {
      setBalancerDesiredState({
        ...balancerDesiredState,
        embeddings: {
          ...embeddingParameters,
          [name]: value,
        },
      });
    },
    [balancerDesiredState, embeddingParameters, setBalancerDesiredState],
  );

  return (
    <>
      <ParameterNumberInput
        description="Max documents per embedding sub-batch (cap; balancer fans out larger requests across agents and chunks beyond this size)"
        name="embedding_batch_size"
        onValue={function (value) {
          setParameter("embedding_batch_size", value);
        }}
        value={embeddingParameters.embedding_batch_size}
      />
      <ParameterSelect
        description="How to combine token embeddings"
        name="pooling_type"
        onValue={function (value) {
          setParameter("pooling_type", value);
        }}
        options={poolingTypes}
        value={embeddingParameters.pooling_type}
      />
    </>
  );
}
