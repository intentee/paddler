import React, { useEffect, useMemo, useState, type ReactNode } from "react";

import { inferenceSocketClient } from "@intentee/paddler-client/inferenceSocketClient";
import {
  TokenGenerationDisabledContext,
  type TokenGenerationDisabledContextValue,
} from "../contexts/TokenGenerationDisabledContext";

export function TokenGenerationDisabledContextProvider({
  children,
  webSocket,
}: {
  children: ReactNode;
  webSocket: null | WebSocket;
}) {
  const [isTokenGenerationDisabled, setIsTokenGenerationDisabled] =
    useState<boolean>(false);

  useEffect(
    function () {
      setIsTokenGenerationDisabled(false);

      if (!webSocket) {
        return;
      }

      const socketClient = inferenceSocketClient({ webSocket });
      const subscription = socketClient.clusterInferenceMode$.subscribe(
        function (clusterInferenceMode) {
          setIsTokenGenerationDisabled(
            "TextGeneration" !== clusterInferenceMode,
          );
        },
      );

      return function () {
        subscription.unsubscribe();
      };
    },
    [webSocket],
  );

  const value = useMemo<TokenGenerationDisabledContextValue>(
    function () {
      return Object.freeze({
        isTokenGenerationDisabled,
      });
    },
    [isTokenGenerationDisabled],
  );

  return (
    <TokenGenerationDisabledContext.Provider value={value}>
      {children}
    </TokenGenerationDisabledContext.Provider>
  );
}
