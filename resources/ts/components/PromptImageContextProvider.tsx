import React, { useMemo, useState, type ReactNode } from "react";

import {
  PromptImageContext,
  type PromptImageContextValue,
} from "../contexts/PromptImageContext";

export function PromptImageContextProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [currentImageDataUri, setCurrentImageDataUri] = useState<null | string>(
    null,
  );
  const [submittedImageDataUri, setSubmittedImageDataUri] = useState<
    null | string
  >(null);

  const isImageAttached = currentImageDataUri !== null;

  const value = useMemo<PromptImageContextValue>(
    function () {
      return Object.freeze({
        currentImageDataUri,
        isImageAttached,
        setCurrentImageDataUri,
        setSubmittedImageDataUri,
        submittedImageDataUri,
      });
    },
    [
      currentImageDataUri,
      isImageAttached,
      setCurrentImageDataUri,
      setSubmittedImageDataUri,
      submittedImageDataUri,
    ],
  );

  return (
    <PromptImageContext.Provider value={value}>
      {children}
    </PromptImageContext.Provider>
  );
}
