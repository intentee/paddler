import React, { useMemo, useState, type ReactNode } from "react";

import {
  PromptThinkingContext,
  type PromptThinkingContextValue,
} from "../contexts/PromptThinkingContext";

export function PromptThinkingContextProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [isThinkingEnabled, setIsThinkingEnabled] = useState<boolean>(true);
  const [submittedIsThinkingEnabled, setSubmittedIsThinkingEnabled] =
    useState<boolean>(true);

  const value = useMemo<PromptThinkingContextValue>(
    function () {
      return Object.freeze({
        isThinkingEnabled,
        setIsThinkingEnabled,
        setSubmittedIsThinkingEnabled,
        submittedIsThinkingEnabled,
      });
    },
    [
      isThinkingEnabled,
      setIsThinkingEnabled,
      setSubmittedIsThinkingEnabled,
      submittedIsThinkingEnabled,
    ],
  );

  return (
    <PromptThinkingContext.Provider value={value}>
      {children}
    </PromptThinkingContext.Provider>
  );
}
