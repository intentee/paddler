import React from "react";
import Markdown from "react-markdown";

import {
  conversationMessage,
  conversationMessage__author,
  conversationMessage__error,
  conversationMessage__response,
  conversationMessage__thoughts,
} from "./ConversationMessage.module.css";

export function ConversationMessage({
  author,
  aiMessage,
  errors,
  isThinking,
  thoughts,
  userMessage,
}: {
  aiMessage: string;
  author: string;
  errors: Array<{
    code: number;
    description: string;
  }>;
  isThinking: boolean;
  thoughts: string;
  userMessage: string;
}) {
  return (
    <div className={conversationMessage}>
      <strong className={conversationMessage__author}>{author}:</strong>
      <div className={conversationMessage__response}>
        <div>
          {isThinking ? (
            "🤔"
          ) : (
            <>
              <Markdown children={aiMessage} />
              {userMessage}
            </>
          )}
        </div>
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
