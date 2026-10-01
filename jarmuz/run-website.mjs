import { jarmuz } from "jarmuz";

export function run({ development, once = false, rustJobs }) {
  const esbuildJob = development ? "esbuild-development" : "esbuild-production";

  jarmuz({
    once,
    pipeline: ["stylelint", "tcm", "tsc", "eslint", esbuildJob, ...rustJobs],
    watch: [
      "paddler_agent",
      "paddler_agent_status",
      "paddler_balancer",
      "paddler_bootstrap",
      "paddler_cache_dir",
      "paddler_cli",
      "paddler_client",
      "paddler_client_javascript",
      "paddler_download_manager",
      "paddler_image_decoder",
      "paddler_inference_parameters",
      "paddler_messaging",
      "paddler_model_source",
      "paddler_request_registry",
      "paddler_state_database",
      "paddler_tool_call_validator",
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
