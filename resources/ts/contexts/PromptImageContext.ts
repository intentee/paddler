import { createContext } from "react";

export type PromptImageContextValue = {
  currentImageDataUri: null | string;
  isImageAttached: boolean;
  setCurrentImageDataUri(this: void, imageDataUri: null | string): void;
  setSubmittedImageDataUri(this: void, imageDataUri: null | string): void;
  submittedImageDataUri: null | string;
};

export const PromptImageContext = createContext<PromptImageContextValue>({
  get currentImageDataUri(): never {
    throw new Error("PromptImageContext not provided");
  },
  get isImageAttached(): never {
    throw new Error("PromptImageContext not provided");
  },
  setCurrentImageDataUri(): never {
    throw new Error("PromptImageContext not provided");
  },
  setSubmittedImageDataUri(): never {
    throw new Error("PromptImageContext not provided");
  },
  get submittedImageDataUri(): never {
    throw new Error("PromptImageContext not provided");
  },
});
