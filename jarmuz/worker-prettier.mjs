import { command } from "jarmuz/job-types";

command(`
  npm exec prettier --
    --plugin=prettier-plugin-organize-imports
    --write
    jarmuz
    paddler_client_javascript/src
    paddler_client_javascript/tests
    resources
    *.mjs
`);
