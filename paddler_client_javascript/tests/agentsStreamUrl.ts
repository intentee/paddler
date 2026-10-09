export function agentsStreamUrl(managementAddress: string): string {
  return `http://${managementAddress}/api/v1/agents/stream`;
}
