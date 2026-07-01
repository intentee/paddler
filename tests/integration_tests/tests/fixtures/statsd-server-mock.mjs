#!/usr/bin/env node

import dgram from "node:dgram";
import { createServer } from "node:http";
import { parseArgs } from "node:util";

const {
  values: { managementPort = "9125", exposePort = "9102" },
} = parseArgs({
  args: process.argv.slice(2),
  options: {
    managementPort: { type: "string" },
    exposePort: { type: "string" },
  },
});

const metrics = {};

const udpServer = dgram.createSocket("udp4");

udpServer.on("message", (msg, rinfo) => {
  const text = msg.toString().trim();
  console.log(`Received: ${text} from ${rinfo.address}:${rinfo.port}`);
  const totalMetrics = text.split("\n");

  for (const oneMetric of totalMetrics) {
    const [name, typeData] = oneMetric.split(":");
    if (!typeData) continue;

    const [rawValue, type] = typeData.split("|");
    const value = parseInt(rawValue);
    const metric = name.replace(/[^a-zA-Z0-9_]/g, "_");

    if (!metrics[metric]) {
      metrics[metric] = { type, value: 0 };
    }

    metrics[metric].value = value;
  }
});

udpServer.on("listening", () => {
  const { port } = udpServer.address();
  console.log(`StatsD server listening on port ${port}`);
});

udpServer.bind(parseInt(managementPort));

const server = createServer(function (req, res) {
  const url = new URL(req.url, `http://${req.headers.host}`);

  if (url.pathname === "/health") {
    if (Object.keys(metrics).length === 0) {
      res.statusCode = 503;
      res.setHeader("Content-Type", "text/plain");
      res.end("Service Unavailable");
      return;
    } else {
      res.statusCode = 200;
      res.setHeader("Content-Type", "text/plain");
      res.end("OK");
    }
  } else if (url.pathname === "/metrics") {
    const output = {};

    for (const [name, data] of Object.entries(metrics)) {
      output[name] = data.value;
    }

    res.statusCode = 200;
    res.setHeader("Content-Type", "application/json");
    res.end(JSON.stringify(output));
  } else {
    res.statusCode = 404;
    res.setHeader("Content-Type", "text/plain");
    res.end("Not Found");
  }
});

server.listen(parseInt(exposePort, 10), function () {
  console.log(`Statsd server is listening on port ${parseInt(exposePort)}`);
});
