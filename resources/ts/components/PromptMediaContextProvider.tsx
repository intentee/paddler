import React, { useMemo, useState, type ReactNode } from "react";

import { type PromptMedia } from "../PromptMedia";
import {
  PromptMediaContext,
  type PromptMediaContextValue,
} from "../contexts/PromptMediaContext";

export function PromptMediaContextProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [currentMedia, setCurrentMedia] = useState<null | PromptMedia>(null);
  const [submittedMedia, setSubmittedMedia] = useState<null | PromptMedia>(
    null,
  );

  const value = useMemo<PromptMediaContextValue>(
    function () {
      return Object.freeze({
        currentMedia,
        setCurrentMedia,
        setSubmittedMedia,
        submittedMedia,
      });
    },
    [currentMedia, setCurrentMedia, setSubmittedMedia, submittedMedia],
  );

  return (
    <PromptMediaContext.Provider value={value}>
      {children}
    </PromptMediaContext.Provider>
  );
}
