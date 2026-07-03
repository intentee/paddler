import React, { useEffect, useMemo, useState, type ReactNode } from "react";

import { inferenceSocketClient } from "@intentee/paddler-client/inferenceSocketClient";
import {
  PromptingDisabledContext,
  type PromptingDisabledContextValue,
} from "../contexts/PromptingDisabledContext";

export function PromptingDisabledContextProvider({
  children,
  webSocket,
}: {
  children: ReactNode;
  webSocket: null | WebSocket;
}) {
  const [isPromptingDisabled, setIsPromptingDisabled] =
    useState<boolean>(false);

  useEffect(
    function () {
      setIsPromptingDisabled(false);

      if (!webSocket) {
        return;
      }

      const socketClient = inferenceSocketClient({ webSocket });
      const subscription = socketClient.clusterPromptingMode$.subscribe(
        function (notification) {
          setIsPromptingDisabled("PromptingDisabled" === notification);
        },
      );

      return function () {
        subscription.unsubscribe();
      };
    },
    [webSocket],
  );

  const value = useMemo<PromptingDisabledContextValue>(
    function () {
      return Object.freeze({
        isPromptingDisabled,
      });
    },
    [isPromptingDisabled],
  );

  return (
    <PromptingDisabledContext.Provider value={value}>
      {children}
    </PromptingDisabledContext.Provider>
  );
}
