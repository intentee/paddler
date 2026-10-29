import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { AgentSchema } from "../../../src/schemas/Agent";

test("AgentSchema parses an agent that is downloading its model", function () {
  const parsed = AgentSchema.parse({
    id: "agent-0",
    name: "agent-0",
    slots_processing: 1,
    status: {
      desired_slots_total: 4,
      download_status: {
        Downloading: {
          downloaded_bytes: 100,
          model_path: "https://example.com/qwen.gguf",
          total_bytes: 400,
        },
      },
      issues: [],
      model_path: "/models/qwen.gguf",
      runtime: {
        Serving: {
          decisions: "NotServed",
          slots_total: 4,
        },
      },
      state_application_status: "Applied",
      uses_chat_template_override: false,
    },
  });

  deepStrictEqual(parsed.status.download_status, {
    Downloading: {
      downloaded_bytes: 100,
      model_path: "https://example.com/qwen.gguf",
      total_bytes: 400,
    },
  });
});
