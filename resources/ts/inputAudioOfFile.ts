import { type InputAudio } from "@intentee/paddler-client/schemas/InputAudio";

const audioFormatOfMimeType: Readonly<Record<string, InputAudio["format"]>> =
  Object.freeze({
    "audio/mp3": "mp3",
    "audio/mpeg": "mp3",
    "audio/wav": "wav",
    "audio/wave": "wav",
    "audio/x-wav": "wav",
  });

function base64Of(bytes: Uint8Array): string {
  let binary = "";

  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }

  return btoa(binary);
}

export async function inputAudioOfFile(audioFile: File): Promise<InputAudio> {
  const format = audioFormatOfMimeType[audioFile.type];

  if (format === undefined) {
    throw new Error(`Unsupported audio file type: ${audioFile.type}`);
  }

  return {
    data: base64Of(new Uint8Array(await audioFile.arrayBuffer())),
    format,
  };
}
