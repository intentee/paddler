import React, { useMemo, useState, type ReactNode } from "react";

import { type BalancerDesiredState } from "@intentee/paddler-client/schemas/BalancerDesiredState";
import {
  BalancerDesiredStateContext,
  type BalancerDesiredStateContextValue,
} from "../contexts/BalancerDesiredStateContext";

export function BalancerDesiredStateContextProvider({
  children,
  defaultBalancerDesiredState,
}: {
  children: ReactNode;
  defaultBalancerDesiredState: BalancerDesiredState;
}) {
  const [balancerDesiredState, setBalancerDesiredState] =
    useState<BalancerDesiredState>(defaultBalancerDesiredState);

  const value = useMemo<BalancerDesiredStateContextValue>(
    function () {
      return Object.freeze({
        balancerDesiredState,
        setBalancerDesiredState,
      });
    },
    [balancerDesiredState, setBalancerDesiredState],
  );

  return (
    <BalancerDesiredStateContext.Provider value={value}>
      {children}
    </BalancerDesiredStateContext.Provider>
  );
}
