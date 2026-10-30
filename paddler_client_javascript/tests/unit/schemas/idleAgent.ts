import type { Agent } from "../../../src/schemas/Agent";

export function idleAgent({
  id,
  name,
}: {
  id: string;
  name: string | null;
}): Agent {
  return {
    id,
    name,
    slots_processing: 0,
    status: {
      desired_slots_total: 1,
      download_status: "NotDownloading",
      issues: [],
      model_path: null,
      runtime: "Idle",
      state_application_status: "Applied",
      uses_chat_template_override: false,
    },
  };
}
