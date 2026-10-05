import { z } from "zod";

export function paddlerBinaryPath(): string {
  return z.string().parse(process.env["PADDLER_BINARY"]);
}
