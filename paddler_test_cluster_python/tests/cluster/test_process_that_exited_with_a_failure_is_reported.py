import asyncio

import pytest

from paddler_test_cluster.error import ProcessExitedWithFailureError
from paddler_test_cluster.paddler_binary_path import paddler_binary_path
from paddler_test_cluster.spawned_process import SpawnedProcess

CLAP_USAGE_ERROR_STATUS = 2


async def test_process_that_exited_with_a_failure_is_reported() -> None:
    process = await asyncio.create_subprocess_exec(
        paddler_binary_path(),
        "--flag-paddler-does-not-have",
        stdout=asyncio.subprocess.DEVNULL,
        stderr=asyncio.subprocess.DEVNULL,
    )
    await process.wait()

    with pytest.raises(ProcessExitedWithFailureError) as exited_with_failure:
        await SpawnedProcess(process).terminate()

    assert exited_with_failure.value.returncode == CLAP_USAGE_ERROR_STATUS
