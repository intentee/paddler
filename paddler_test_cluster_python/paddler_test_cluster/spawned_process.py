import asyncio
from datetime import timedelta

from paddler_test_cluster.error import (
    ProcessDidNotExitError,
    ProcessExitedWithFailureError,
)

TRZCINA_DEFAULT_COOPERATIVE_DEADLINE = timedelta(seconds=10)
TRZCINA_DEFAULT_ABORT_DEADLINE = timedelta(seconds=10)
PADDLER_SHUTDOWN_BOUND = (
    TRZCINA_DEFAULT_COOPERATIVE_DEADLINE + TRZCINA_DEFAULT_ABORT_DEADLINE
)


class SpawnedProcess:
    def __init__(self, process: asyncio.subprocess.Process) -> None:
        self._process = process

    @property
    def pid(self) -> int:
        return self._process.pid

    async def terminate(self) -> None:
        if self._process.returncode is None:
            self._process.terminate()

        try:
            returncode = await asyncio.wait_for(
                self._process.wait(),
                PADDLER_SHUTDOWN_BOUND.total_seconds(),
            )
        except TimeoutError as timeout_error:
            self._process.kill()
            await self._process.wait()

            raise ProcessDidNotExitError(self._process.pid) from timeout_error

        if returncode != 0:
            raise ProcessExitedWithFailureError(self._process.pid, returncode)
