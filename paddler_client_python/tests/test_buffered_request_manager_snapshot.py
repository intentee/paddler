from paddler_client.buffered_request_manager_snapshot import (
    BufferedRequestManagerSnapshot,
)


def test_buffered_request_manager_snapshot_deserialization() -> None:
    snapshot = BufferedRequestManagerSnapshot.model_validate(
        {"buffered_requests_current": 12}
    )

    assert snapshot.buffered_requests_current == 12
