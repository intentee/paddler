import { throws } from "node:assert/strict";
import { test } from "node:test";

import { ConversationMessageSchema } from "../../../src/schemas/ConversationMessage";

test("ConversationMessageSchema rejects a message without its kind", function () {
  throws(function () {
    ConversationMessageSchema.parse({ content: "Hello", role: "user" });
  });
});
