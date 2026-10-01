import { deepStrictEqual, throws } from "node:assert/strict";
import { test } from "node:test";

import { AgentSchema } from "../../../src/schemas/Agent";

test("parses an agent that is downloading its model", function () {
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
      slots_total: 4,
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

test("rejects an unknown state_application_status", function () {
  throws(function () {
    AgentSchema.parse({
      id: "agent-x",
      name: null,
      slots_processing: 0,
      status: {
        desired_slots_total: 1,
        download_status: "NotDownloading",
        issues: [],
        model_path: null,
        slots_total: 1,
        state_application_status: "Unknown",
        uses_chat_template_override: false,
      },
    });
  });
});
