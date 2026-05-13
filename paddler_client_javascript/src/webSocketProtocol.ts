export function webSocketProtocol(httpProtocol: string): string {
  return httpProtocol === "https:" ? "wss:" : "ws:";
}
