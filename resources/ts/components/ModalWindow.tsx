import React, { useCallback, type MouseEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";

import iconClose from "../../icons/close.svg";
import {
  modalWindow,
  modalWindow__closeButton,
  modalWindow__content,
  modalWindow__titleBar,
  modalWindow__titleBar__title,
  modalWindowBackdrop,
} from "./ModalWindow.module.css";

export function ModalWindow({
  children,
  confirmCloseMessage,
  onClose,
  title,
}: {
  children: ReactNode;
  confirmCloseMessage?: string;
  onClose(this: void): void;
  title: string;
}) {
  const onCloseButtonClick = useCallback(
    function (evt: MouseEvent<HTMLButtonElement>) {
      evt.preventDefault();

      if (confirmCloseMessage && !window.confirm(confirmCloseMessage)) {
        return;
      }

      onClose();
    },
    [confirmCloseMessage, onClose],
  );

  return createPortal(
    <div className={modalWindowBackdrop}>
      <div className={modalWindow}>
        <div className={modalWindow__titleBar}>
          <div className={modalWindow__titleBar__title}>{title}</div>
          <button
            className={modalWindow__closeButton}
            onClick={onCloseButtonClick}
          >
            <img src={iconClose} alt="Close" />
          </button>
        </div>
        <div className={modalWindow__content}>{children}</div>
      </div>
    </div>,
    document.body,
  );
}
