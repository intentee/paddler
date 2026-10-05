from typing import Any, cast

from pydantic import BaseModel, ConfigDict, model_validator

from paddler_client.error import InvalidModelDownloadStatusError

DOWNLOADING_VARIANTS = ("Downloading", "DownloadingWithUnknownSize")


class ModelDownloadStatus(BaseModel):
    model_config = ConfigDict(frozen=True)

    variant: str
    downloaded_bytes: int | None = None
    model_path: str | None = None
    total_bytes: int | None = None

    @model_validator(mode="before")
    @classmethod
    def from_serde(cls, data: Any) -> dict[str, Any]:
        if data == "NotDownloading":
            return {"variant": "NotDownloading"}

        if isinstance(data, dict):
            typed_data = cast("dict[str, Any]", data)

            if "variant" in typed_data:
                return typed_data

            if len(typed_data) == 1:
                variant, progress = next(iter(typed_data.items()))

                if variant in DOWNLOADING_VARIANTS and isinstance(progress, dict):
                    return {"variant": variant, **cast("dict[str, Any]", progress)}

        raise InvalidModelDownloadStatusError(data)
