import { createContext } from "react";

import { type BalancerDesiredState } from "@intentee/paddler-client/schemas/BalancerDesiredState";

export type BalancerDesiredStateContextValue = {
  balancerDesiredState: BalancerDesiredState;
  setBalancerDesiredState(
    this: void,
    balancerDesiredState: BalancerDesiredState,
  ): void;
};

export const BalancerDesiredStateContext =
  createContext<BalancerDesiredStateContextValue>({
    get balancerDesiredState(): never {
      throw new Error("BalancerDesiredStateContext not provided");
    },
    setBalancerDesiredState(): never {
      throw new Error("BalancerDesiredStateContext not provided");
    },
  });
