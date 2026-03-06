import { type ConversationMessageContentPart } from "./ConversationMessageContentPart.type";

export type ConversationMessage = {
  role: string;
  content: string | ConversationMessageContentPart[];
};
