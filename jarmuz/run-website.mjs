import { jarmuz } from "jarmuz";

export function run({ development, once = false, rustJobs }) {
  const esbuildJob = development ? "esbuild-development" : "esbuild-production";

  jarmuz({
    once,
    pipeline: ["stylelint", "tcm", "tsc", "eslint", esbuildJob, ...rustJobs],
    watch: [
      "paddler",
      "paddler_client",
      "paddler_client_javascript",
      "paddler_types",
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
      case matches("paddler/templates/**/*.html"):
      case matches("**/*.rs"):
        for (const job of rustJobs) {
          schedule(job);
        }
        return;
    }
  });
}
