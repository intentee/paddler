import type { HuggingFaceModelReference } from "./schemas/HuggingFaceModelReference";

export function extractHuggingFaceUrlParts({
  pathname,
}: URL): HuggingFaceModelReference {
  const segments = pathname.split("/").filter(function (segment) {
    return segment.length > 0;
  });

  if (segments.length < 5) {
    throw new Error(`Invalid Hugging Face URL format: ${pathname}`);
  }

  const [owner, repo, resourceKind, revision, ...filenameSegments] = segments;

  if (
    owner === undefined
    || repo === undefined
    || resourceKind === undefined
    || revision === undefined
  ) {
    throw new Error(`Invalid Hugging Face URL format: ${pathname}`);
  }

  if (resourceKind !== "blob" && resourceKind !== "resolve") {
    throw new Error(`Invalid Hugging Face URL format: ${pathname}`);
  }

  if (filenameSegments.length < 1) {
    throw new Error(`Invalid Hugging Face URL format: ${pathname}`);
  }

  return {
    filename: filenameSegments.join("/"),
    repo_id: `${owner}/${repo}`,
    revision,
  };
}
