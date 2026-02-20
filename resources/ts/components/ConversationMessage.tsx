import React from "react";
import Markdown from "react-markdown";
import remarkBreaks from "remark-breaks";

import {
  conversationMessage,
  conversationMessage__author,
  conversationMessage__error,
  conversationMessage__response,
  conversationMessage__thoughts,
} from "./ConversationMessage.module.css";

export function ConversationMessage({
  author,
  errors,
  isThinking,
  response,
  thoughts,
}: {
  author: string;
  errors: Array<{
    code: number;
    description: string;
  }>;
  isThinking: boolean;
  response: string;
  thoughts: string;
}) {
  return (
    <div className={conversationMessage}>
      <strong className={conversationMessage__author}>{author}:</strong>
      <div className={conversationMessage__response}>
        <div>
          {isThinking ? (
            "🤔"
          ) : (
            <Markdown remarkPlugins={[remarkBreaks]}>
              {author === "You"
                ? response.replace(/\n\s*\n/g, "\n&NewLine;\n")
                : response}
            </Markdown>
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
