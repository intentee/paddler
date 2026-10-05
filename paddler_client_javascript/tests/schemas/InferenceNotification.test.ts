import { strictEqual, throws } from "node:assert/strict";
import { test } from "node:test";

import { InferenceNotificationSchema } from "../../src/schemas/InferenceNotification";

test("parses a cluster token-generation-mode notification frame", function () {
  const parsed = InferenceNotificationSchema.parse({
    Notification: "TokenGenerationEnabled",
  });

  strictEqual(parsed.Notification, "TokenGenerationEnabled");
});

test("rejects an unknown notification value", function () {
  throws(function () {
    InferenceNotificationSchema.parse({ Notification: "SomethingElse" });
  });
});
