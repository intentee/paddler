import { strictEqual, throws } from "node:assert/strict";
import { test } from "node:test";

import { AgentSchema } from "../../src/schemas/Agent";

test("parses a fully populated agent payload", function () {
  const parsed = AgentSchema.parse({
    desired_slots_total: 4,
    download_current: 0,
    download_filename: null,
    download_total: 0,
    id: "agent-0",
    issues: [],
    model_path: "/models/qwen.gguf",
    name: "agent-0",
    slots_processing: 1,
    slots_total: 4,
    state_application_status: "Applied",
    uses_chat_template_override: false,
  });

  strictEqual(parsed.id, "agent-0");
  strictEqual(parsed.state_application_status, "Applied");
});

test("rejects an unknown state_application_status", function () {
  throws(function () {
    AgentSchema.parse({
      desired_slots_total: 1,
      download_current: 0,
      download_filename: null,
      download_total: 0,
      id: "agent-x",
      issues: [],
      model_path: null,
      name: null,
      slots_processing: 0,
      slots_total: 1,
      state_application_status: "Unknown",
      uses_chat_template_override: false,
    });
  });
});
