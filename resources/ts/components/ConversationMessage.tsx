import React, { type ReactNode } from "react";

import {
  conversationMessage,
  conversationMessage__author,
  conversationMessage__error,
  conversationMessage__response,
  conversationMessage__thoughts,
} from "./ConversationMessage.module.css";

export function ConversationMessage({
  author,
  children,
  errors,
  isThinking,
  thoughts,
}: {
  author: string;
  children: ReactNode;
  errors: Array<{
    code: number;
    description: string;
  }>;
  isThinking: boolean;
  thoughts: string;
}) {
  return (
    <div className={conversationMessage}>
      <strong className={conversationMessage__author}>{author}:</strong>
      <div className={conversationMessage__response}>
        <div>{isThinking ? "🤔" : children}</div>
        {errors.map(function ({ code, description }, index) {
          return (
            <div className={conversationMessage__error} key={index}>
              <strong>Error {code}:</strong> {description}
            </div>
          );
        })}
      </div>
      <div className={conversationMessage__thoughts}>{thoughts}</div>
    </div>
  );
}
