export class ProcessDidNotExitError extends Error {
  override name = "ProcessDidNotExitError";

  constructor(public readonly pid: number) {
    super(`Process ${pid} did not exit within its release window`);
  }
}
