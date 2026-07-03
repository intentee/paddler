import { strictEqual, throws } from "node:assert/strict";
import { test } from "node:test";

import { InferenceNotificationSchema } from "../../src/schemas/InferenceNotification";

test("parses a cluster prompting-mode notification frame", function () {
  const parsed = InferenceNotificationSchema.parse({
    Notification: "PromptingEnabled",
  });

  strictEqual(parsed.Notification, "PromptingEnabled");
});

test("rejects an unknown notification value", function () {
  throws(function () {
    InferenceNotificationSchema.parse({ Notification: "SomethingElse" });
  });
});
