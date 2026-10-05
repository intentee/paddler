#!/usr/bin/env node

import { jarmuz } from "jarmuz";

jarmuz({
  once: true,
  pipeline: ["cargo-fmt", "prettier"],
  watch: [
    "jarmuz",
    "paddler_agent",
    "paddler_agent_status",
    "paddler_balancer",
    "paddler_bootstrap",
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
    "paddler_opencode_tests",
    "paddler_request_registry",
    "paddler_state_database",
    "paddler_test_cluster_harness",
    "paddler_tests",
    "paddler_tool_call_validator",
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
