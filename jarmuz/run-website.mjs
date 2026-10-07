import { jarmuz } from "jarmuz";

export function run({ development, once = false, rustJobs }) {
  const esbuildJob = development ? "esbuild-development" : "esbuild-production";

  jarmuz({
    once,
    pipeline: ["stylelint", "tcm", "tsc", "eslint", esbuildJob, ...rustJobs],
    watch: [
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
      "paddler_client",
      "paddler_client_javascript",
      "paddler_download_manager",
      "paddler_image_decoder",
      "paddler_inference_parameters",
      "paddler_messaging",
      "paddler_model_source",
      "paddler_openai_translation",
      "paddler_request_registry",
      "paddler_service_thread",
      "paddler_state_database",
      "paddler_tool_call_validator",
      "paddler_typesafe_translation",
      "resources",
    ],
  }).decide(function ({ matches, schedule }) {
    if (matches("resources/**/*.css")) {
      schedule("stylelint");
    }

    switch (true) {
      case matches("resources/**/*.{ts,tsx}"):
      case matches("paddler_client_javascript/src/**/*.ts"):
        schedule("tsc");
        schedule("eslint");
        break;
      case matches("resources/css/**/*.css"):
        schedule("tcm");
        schedule(esbuildJob);
        return;
      case matches("paddler_balancer/templates/**/*.html"):
      case matches("**/*.rs"):
        for (const job of rustJobs) {
          schedule(job);
        }
        return;
    }
  });
}
