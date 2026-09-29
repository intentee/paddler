import { JsonError } from "./JsonError";

function parseLine(line: string): unknown {
  try {
    return JSON.parse(line);
  } catch (error: unknown) {
    throw new JsonError(`Failed to parse NDJSON line: ${String(error)}`, line);
  }
}

function parseLines(text: string): unknown[] {
  return text
    .split("\n")
    .map(function (line) {
      return line.trim();
    })
    .filter(function (line) {
      return line.length > 0;
    })
    .map(parseLine);
}

export class NdjsonDecoder {
  #unterminatedLine = "";

  push(text: string): unknown[] {
    const buffered = this.#unterminatedLine + text;
    const lastNewlineIndex = buffered.lastIndexOf("\n");

    this.#unterminatedLine = buffered.slice(lastNewlineIndex + 1);

    return parseLines(buffered.slice(0, lastNewlineIndex + 1));
  }

  finish(): unknown[] {
    const unterminatedLine = this.#unterminatedLine;

    this.#unterminatedLine = "";

    return parseLines(unterminatedLine);
  }
}
