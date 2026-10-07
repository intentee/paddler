#!/usr/bin/env node

import { jarmuz } from "jarmuz";

jarmuz({
  once: true,
  pipeline: ["cargo-fmt", "prettier"],
  watch: [
    "jarmuz",
    "paddler_agent",
    "paddler_agent_decision",
    "paddler_agent_embeddings",
    "paddler_agent_pointer_head",
    "paddler_agent_runner",
    "paddler_agent_runtime",
    "paddler_agent_status",
    "paddler_agent_text_generation",
    "paddler_balancer",
    "paddler_balancer_runner",
    "paddler_cache_dir",
    "paddler_cli",
    "paddler_cli_tests",
    "paddler_client",
    "paddler_client_javascript",
    "paddler_download_manager",
    "paddler_gui",
    "paddler_gui_tests",
    "paddler_image_decoder",
    "paddler_inference_parameters",
    "paddler_local_http_fixture",
    "paddler_messaging",
    "paddler_model_source",
    "paddler_openai_response_format_validator",
    "paddler_openai_translation",
    "paddler_opencode_tests",
    "paddler_request_registry",
    "paddler_service_thread",
    "paddler_state_database",
    "paddler_test_cluster_harness",
    "paddler_tests",
    "paddler_tool_call_validator",
    "paddler_typesafe_translation",
    "resources",
  ],
}).decide(function ({ matches, schedule }) {
  switch (true) {
    case matches("**/*.css"):
    case matches("**/*.mjs"):
    case matches("**/*.ts"):
    case matches("**/*.tsx"):
      schedule("prettier");
      break;
    case matches("**/*.rs"):
      schedule("cargo-fmt");
      break;
  }
});
