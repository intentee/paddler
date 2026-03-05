import { type Observable } from "rxjs";

import { type ConversationMessage } from "./ConversationMessage.type";
import { type InferenceServiceGenerateTokensResponse } from "./schemas/InferenceServiceGenerateTokensResponse";

export interface InferenceSocketClient {
  continueConversation(params: {
    messages: ConversationMessage[];
  }): Observable<InferenceServiceGenerateTokensResponse>;
}
