import React, { useCallback, useContext } from "react";

import { PromptMediaContext } from "../contexts/PromptMediaContext";
import { inputAudioOfFile } from "../inputAudioOfFile";
import { readFileAsDataUri } from "../readFileAsDataUri";
import { ConversationPromptInputMediaButton } from "./ConversationPromptInputMediaButton";

import iconMic from "../../icons/mic.svg";

export function ConversationPromptInputAudioButton() {
  const { setCurrentMedia } = useContext(PromptMediaContext);

  const onFile = useCallback(
    function (audioFile: File) {
      void Promise.all([
        inputAudioOfFile(audioFile),
        readFileAsDataUri(audioFile),
      ]).then(function ([audio, dataUri]) {
        setCurrentMedia({ audio, dataUri, kind: "audio" });
      });
    },
    [setCurrentMedia],
  );

  return (
    <ConversationPromptInputMediaButton
      accept="audio/mpeg,audio/wav"
      iconAlt="Add audio"
      iconSrc={iconMic}
      onFile={onFile}
    />
  );
}
