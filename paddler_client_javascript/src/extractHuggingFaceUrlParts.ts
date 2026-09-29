import { InvalidHuggingFaceUrlError } from "./InvalidHuggingFaceUrlError";
import type { HuggingFaceModelReference } from "./schemas/HuggingFaceModelReference";

export function extractHuggingFaceUrlParts({
  pathname,
}: URL): HuggingFaceModelReference {
  const [owner, repo, resourceKind, revision, ...filenameSegments] = pathname
    .split("/")
    .filter(function (segment) {
      return segment.length > 0;
    });

  if (
    owner === undefined ||
    repo === undefined ||
    revision === undefined ||
    (resourceKind !== "blob" && resourceKind !== "resolve") ||
    filenameSegments.length === 0
  ) {
    throw new InvalidHuggingFaceUrlError(pathname);
  }

  return {
    filename: filenameSegments.join("/"),
    repo_id: `${owner}/${repo}`,
    revision,
  };
}
