import pytest
from pydantic import ValidationError

from paddler_client.error import InvalidModelDownloadStatusError
from paddler_client.model_download_status import ModelDownloadStatus
from tests.unit.validation_error_cause import validation_error_cause


def test_model_download_status_reads_a_download_of_known_size() -> None:
    download_status = ModelDownloadStatus.model_validate(
        {
            "Downloading": {
                "downloaded_bytes": 100,
                "model_path": "https://example.com/model.gguf",
                "total_bytes": 1000,
            }
        }
    )

    assert download_status == ModelDownloadStatus(
        variant="Downloading",
        downloaded_bytes=100,
        model_path="https://example.com/model.gguf",
        total_bytes=1000,
    )


def test_model_download_status_reads_no_download() -> None:
    download_status = ModelDownloadStatus.model_validate("NotDownloading")

    assert download_status == ModelDownloadStatus(variant="NotDownloading")


def test_model_download_status_rejects_an_unknown_variant() -> None:
    with pytest.raises(ValidationError) as rejection:
        ModelDownloadStatus.model_validate({"Paused": {"downloaded_bytes": 100}})

    assert isinstance(
        validation_error_cause(rejection.value), InvalidModelDownloadStatusError
    )


def test_model_download_status_rejects_a_download_without_its_progress() -> None:
    with pytest.raises(ValidationError) as rejection:
        ModelDownloadStatus.model_validate("Downloading")

    assert isinstance(
        validation_error_cause(rejection.value), InvalidModelDownloadStatusError
    )


def test_model_download_status_rejects_a_status_naming_two_variants() -> None:
    with pytest.raises(ValidationError) as rejection:
        ModelDownloadStatus.model_validate(
            {
                "Downloading": {
                    "downloaded_bytes": 1,
                    "model_path": "a",
                    "total_bytes": 2,
                },
                "NotDownloading": {},
            }
        )

    assert isinstance(
        validation_error_cause(rejection.value), InvalidModelDownloadStatusError
    )
