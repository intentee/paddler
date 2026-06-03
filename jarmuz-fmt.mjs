#!/usr/bin/env node

import { jarmuz } from "jarmuz";

jarmuz({
  once: true,
  pipeline: ["cargo-fmt", "prettier"],
  watch: [
    "jarmuz",
    "paddler_agent",
    "paddler_balancer",
    "paddler_bootstrap",
    "paddler_cache_dir",
    "paddler_cli",
    "paddler_cli_tests",
    "paddler_client",
    "paddler_client_cli",
    "paddler_cluster_harness",
    "paddler_download_manager",
    "paddler_gui",
    "paddler_messaging",
    "paddler_state_conversion",
    "paddler_tests",
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
