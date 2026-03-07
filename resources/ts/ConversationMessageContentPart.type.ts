export type ConversationMessageContentPart =
  | { type: "text"; text: string }
  | { type: "image_url"; image_url: { url: string } };
