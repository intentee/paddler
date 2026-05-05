import React, {
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type MouseEvent,
} from "react";

import { ChatTemplateContext } from "../contexts/ChatTemplateContext";
import { type ChatTemplate } from "@intentee/paddler-client/schemas/ChatTemplate";
import { CodeEditor } from "./CodeEditor";
import { ModalWindow } from "./ModalWindow";

import iconSave from "../../icons/save.svg";
import {
  chatTemplateEditButton,
  chatTemplateEditButton__modal,
  chatTemplateEditButton__modal__content,
  chatTemplateEditButton__modal__toolbar,
  chatTemplateEditButton__modal__toolbarButton,
} from "./ChatTemplateEditButton.module.css";

function toDefaultContent(chatTemplate: null | ChatTemplate): string {
  return chatTemplate?.content || "";
}

export function ChatTemplateEditButton() {
  const [isEditing, setIsEditing] = useState(false);
  const {
    chatTemplateOverride,
    setChatTemplateOverrideContent,
    useChatTemplateOverride,
  } = useContext(ChatTemplateContext);
  const [temporaryContent, setTemporaryContent] = useState(
    toDefaultContent(chatTemplateOverride),
  );

  useEffect(
    function () {
      setTemporaryContent(toDefaultContent(chatTemplateOverride));
    },
    [chatTemplateOverride, setTemporaryContent],
  );

  const onClick = useCallback(
    function (evt: MouseEvent<HTMLButtonElement>) {
      evt.preventDefault();

      setIsEditing(true);
    },
    [setIsEditing],
  );

  const onClose = useCallback(
    function () {
      setTemporaryContent(toDefaultContent(chatTemplateOverride));
      setIsEditing(false);
    },
    [chatTemplateOverride, setIsEditing, setTemporaryContent],
  );

  const onSaveClick = useCallback(
    function (evt: MouseEvent<HTMLButtonElement>) {
      evt.preventDefault();

      setChatTemplateOverrideContent(temporaryContent);
      setIsEditing(false);
    },
    [setIsEditing, setChatTemplateOverrideContent, temporaryContent],
  );

  const shouldConfirmClose = useMemo(
    function () {
      return toDefaultContent(chatTemplateOverride) !== temporaryContent;
    },
    [chatTemplateOverride, temporaryContent],
  );

  return (
    <>
      <button
        className={chatTemplateEditButton}
        disabled={!useChatTemplateOverride}
        onClick={onClick}
      >
        Edit chat template
      </button>
      {isEditing && (
        <ModalWindow
          confirmCloseMessage={
            shouldConfirmClose
              ? "You have unsaved changes. Are you sure you want to close?"
              : undefined
          }
          onClose={onClose}
          title="Edit chat template"
        >
          <div className={chatTemplateEditButton__modal}>
            <div className={chatTemplateEditButton__modal__toolbar}>
              <button
                className={chatTemplateEditButton__modal__toolbarButton}
                onClick={onSaveClick}
              >
                <img src={iconSave} alt="Save" />
                Save changes
              </button>
            </div>
            <div className={chatTemplateEditButton__modal__content}>
              <CodeEditor
                editable
                onChange={setTemporaryContent}
                value={temporaryContent}
              />
            </div>
          </div>
        </ModalWindow>
      )}
    </>
  );
}
