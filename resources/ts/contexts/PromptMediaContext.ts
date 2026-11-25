import { createContext } from "react";

import { type PromptMedia } from "../PromptMedia";

export type PromptMediaContextValue = {
  currentMedia: null | PromptMedia;
  setCurrentMedia(this: void, media: null | PromptMedia): void;
  setSubmittedMedia(this: void, media: null | PromptMedia): void;
  submittedMedia: null | PromptMedia;
};

export const PromptMediaContext = createContext<PromptMediaContextValue>({
  get currentMedia(): never {
    throw new Error("PromptMediaContext not provided");
  },
  setCurrentMedia(): never {
    throw new Error("PromptMediaContext not provided");
  },
  setSubmittedMedia(): never {
    throw new Error("PromptMediaContext not provided");
  },
  get submittedMedia(): never {
    throw new Error("PromptMediaContext not provided");
  },
});
