import React, { useContext } from "react";

import { TokenGenerationDisabledContext } from "../contexts/TokenGenerationDisabledContext";

import { tokenGenerationDisabledNotice } from "./TokenGenerationDisabledNotice.module.css";

export function TokenGenerationDisabledNotice() {
  const { isTokenGenerationDisabled } = useContext(
    TokenGenerationDisabledContext,
  );

  if (!isTokenGenerationDisabled) {
    return null;
  }

  return (
    <div className={tokenGenerationDisabledNotice}>
      Token generation is disabled while the embeddings are enabled. Learn more
      in the{" "}
      <a
        href="https://paddler.intentee.com/docs/starting-out/generating-tokens-and-embeddings/"
        target="_blank"
      >
        documentation
      </a>
      .
    </div>
  );
}
