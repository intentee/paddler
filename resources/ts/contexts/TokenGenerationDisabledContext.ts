import { createContext } from "react";

export type TokenGenerationDisabledContextValue = {
  isTokenGenerationDisabled: boolean;
};

export const TokenGenerationDisabledContext =
  createContext<TokenGenerationDisabledContextValue>({
    get isTokenGenerationDisabled(): never {
      throw new Error("TokenGenerationDisabledContext not provided");
    },
  });
