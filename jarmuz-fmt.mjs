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
    "paddler_agent_runtime",
    "paddler_agent_runtime_tests",
    "paddler_agent_status",
    "paddler_agent_tests",
    "paddler_agent_text_generation",
    "paddler_balancer",
    "paddler_balancer_desired_state_fixture",
    "paddler_blocking_thread_fixture",
    "paddler_bootstrap",
    "paddler_bootstrap_tests",
    "paddler_cache_dir",
    "paddler_chunked_response_fixture",
    "paddler_cli",
    "paddler_cli_tests",
    "paddler_client",
    "paddler_client_javascript",
    "paddler_dashboard_assets",
    "paddler_download_manager",
    "paddler_download_manager_tests",
    "paddler_frame_writer",
    "paddler_generation_result_fixture",
    "paddler_image_decoder",
    "paddler_inference_parameters",
    "paddler_local_http_fixture",
    "paddler_messaging",
    "paddler_model_source",
    "paddler_openai_response_format_validator",
    "paddler_openai_translation",
    "paddler_openai_translation_tests",
    "paddler_opencode_tests",
    "paddler_outbox",
    "paddler_request_registry",
    "paddler_sha256_hex",
    "paddler_spawn_resuming_panic",
    "paddler_state_database",
    "paddler_tcp_buffer_limits",
    "paddler_test_cluster_harness",
    "paddler_tests",
    "paddler_tool_call_validator",
    "paddler_typesafe_translation",
    "paddler_typesafe_translation_tests",
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
