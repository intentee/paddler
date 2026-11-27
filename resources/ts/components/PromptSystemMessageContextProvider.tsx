import React, { useMemo, useState, type ReactNode } from "react";

import {
  PromptSystemMessageContext,
  type PromptSystemMessageContextValue,
} from "../contexts/PromptSystemMessageContext";

export function PromptSystemMessageContextProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [
    submittedIsSystemMessageIncluded,
    setSubmittedIsSystemMessageIncluded,
  ] = useState<boolean>(false);

  const value = useMemo<PromptSystemMessageContextValue>(
    function () {
      return Object.freeze({
        setSubmittedIsSystemMessageIncluded,
        submittedIsSystemMessageIncluded,
      });
    },
    [setSubmittedIsSystemMessageIncluded, submittedIsSystemMessageIncluded],
  );

  return (
    <PromptSystemMessageContext.Provider value={value}>
      {children}
    </PromptSystemMessageContext.Provider>
  );
}
