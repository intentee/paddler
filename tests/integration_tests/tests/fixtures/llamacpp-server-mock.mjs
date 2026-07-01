#!/usr/bin/env node

import { appendFile } from "node:fs";
import { createServer } from "node:http";
import { parseArgs } from "node:util";

const {
  values: { completionResponseDelay, logFile, name, port, slots },
} = parseArgs({
  args: process.argv.slice(2),
  options: {
    completionResponseDelay: { type: "string" },
    logFile: { type: "string" },
    name: { type: "string" },
    port: { type: "string" },
    slots: { type: "string" },
  },
});

const completionResponseDelayInt = parseInt(completionResponseDelay, 10);
const portInt = parseInt(port, 10);
const slotsInt = parseInt(slots, 10);
const slotsStatuses = [];

for (let i = 0; i < slotsInt; i += 1) {
  slotsStatuses.push({
    id: i,
    is_processing: false,
  });
}

const server = createServer(function (req, res) {
  if (req.url === "/chat/completions") {
    const requestName = req.headers["x-request-name"];

    if (!requestName) {
      res.statusCode = 400;
      res.setHeader("Content-Type", "application/json");
      res.end('{"error":"Missing x-request-name header"}');
      return;
    }

    const slot = slotsStatuses[0];
    slot.is_processing = true;

    setTimeout(function () {
      appendFile(logFile, `${name};${requestName}\n`, function (err) {
        slot.is_processing = false;
        if (err) {
          res.statusCode = 500;
          res.setHeader("Content-Type", "text/plain");
          res.end(String(err));
        } else {
          res.statusCode = 200;
          res.setHeader("Content-Type", "application/json");
          res.end("{}");
        }
      });
    }, completionResponseDelayInt);
  } else if (req.url === "/health") {
    res.statusCode = 200;
    res.setHeader("Content-Type", "text/plain");
    res.end("OK");
  } else if (req.url === "/slots") {
    res.statusCode = 200;
    res.setHeader("Content-Type", "application/json");
    res.end(JSON.stringify(slotsStatuses));
  } else {
    res.statusCode = 404;
    res.setHeader("Content-Type", "application/json");
    res.end('{"status":"not found"}');
  }
});

server.listen(portInt, function () {
  console.log(
    `Server ${name} is listening on port ${portInt} (with ${slotsInt} slots)`,
  );
});
