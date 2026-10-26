import React, { useMemo, useState, type ReactNode } from "react";

import {
  PromptContext,
  type PromptContextValue,
} from "../contexts/PromptContext";

export function PromptContextProvider({ children }: { children: ReactNode }) {
  const [currentPrompt, setCurrentPrompt] = useState<string>("");
  const [submittedPrompt, setSubmittedPrompt] = useState<{
    prompt: string | null;
    version: number;
  }>({
    prompt: null,
    version: 0,
  });

  const value = useMemo<PromptContextValue>(
    function () {
      return Object.freeze({
        currentPrompt,
        isCurrentPromptEmpty: currentPrompt.trim() === "",
        setCurrentPrompt,
        setSubmittedPrompt(prompt: string | null) {
          setCurrentPrompt("");
          setSubmittedPrompt(function ({ version }) {
            return Object.freeze({
              prompt,
              version: version + 1,
            });
          });
        },
        submittedPrompt: submittedPrompt.prompt,
        version: submittedPrompt.version,
      });
    },
    [
      currentPrompt,
      setCurrentPrompt,
      setSubmittedPrompt,
      submittedPrompt.prompt,
      submittedPrompt.version,
    ],
  );

  return (
    <PromptContext.Provider value={value}>{children}</PromptContext.Provider>
  );
}
