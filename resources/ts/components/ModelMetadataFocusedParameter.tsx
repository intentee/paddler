import React, { useCallback, useContext, type MouseEvent } from "react";

import {
  ModelMetadataContext,
  type FocusedMetadataParameter,
} from "../contexts/ModelMetadataContext";
import { CodeEditor } from "./CodeEditor";

import iconArrowBack from "../../icons/arrow_back.svg";
import {
  modelMetadataFocusedParameter,
  modelMetadataFocusedParameter__backButton,
  modelMetadataFocusedParameter__backPanel,
  modelMetadataFocusedParameter__content,
} from "./ModelMetadataFocusedParameter.module.css";

export function ModelMetadataFocusedParameter({
  focusedMetadataParameter: { metadataValue },
}: {
  focusedMetadataParameter: FocusedMetadataParameter;
}) {
  const { setFocusedMetadataParameter } = useContext(ModelMetadataContext);

  const onBackClick = useCallback(
    function (evt: MouseEvent<HTMLButtonElement>) {
      evt.preventDefault();

      setFocusedMetadataParameter(void 0);
    },
    [setFocusedMetadataParameter],
  );

  return (
    <div className={modelMetadataFocusedParameter}>
      <div className={modelMetadataFocusedParameter__backPanel}>
        <button
          className={modelMetadataFocusedParameter__backButton}
          onClick={onBackClick}
        >
          <img src={iconArrowBack} alt="Back" />
          Back to all parameters
        </button>
      </div>
      <div className={modelMetadataFocusedParameter__content}>
        <CodeEditor editable={false} value={metadataValue} />
      </div>
    </div>
  );
}
