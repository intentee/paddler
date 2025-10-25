import notifier from "node-notifier";
import { temporaryFile } from "tempy";

import { spawner } from "jarmuz/job-types";

const stateDatabase = temporaryFile({
  extension: "json",
  prefix: "worker-paddler-state-database-",
});

spawner(async function ({ buildId, command }) {
  notifier.notify({
    title: "paddler",
    message: `Build ${buildId} finished`,
    timeout: 1,
  });

  const results = await Promise.all([
    command(`
      target/debug/paddler balancer
        --inference-addr 127.0.0.1:8061
        --management-addr 127.0.0.1:8060
        --state-database file://${stateDatabase}
        --web-admin-panel-addr 127.0.0.1:8062
    `),
    command(`
      target/debug/paddler agent
        --management-addr 127.0.0.1:8060
        --name agent-1
        --slots 4
    `),
    command(`
      target/debug/paddler agent
        --management-addr 127.0.0.1:8060
        --name agent-2
        --slots 4
    `),
    command(`
      target/debug/paddler agent
        --management-addr 127.0.0.1:8060
        --name agent-3
        --slots 2
    `),
  ]);

  for (const result of results) {
    if (!result) {
      return false;
    }
  }
});
