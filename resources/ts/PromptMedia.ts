import { type InputAudio } from "@intentee/paddler-client/schemas/InputAudio";

export type PromptMedia =
  | { audio: InputAudio; dataUri: string; kind: "audio" }
  | { dataUri: string; kind: "image" };
