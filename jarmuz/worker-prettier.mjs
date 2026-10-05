import { command } from "jarmuz/job-types";

command(`
  npm exec prettier --
    --plugin=prettier-plugin-organize-imports
    --write
    jarmuz
    resources
    *.mjs
`);
