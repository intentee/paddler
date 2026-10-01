import os
import signal

import pytest

from paddler_test_cluster.error import ProcessDidNotExitError
from paddler_test_cluster.spawned_balancer import SpawnedBalancer


async def test_process_that_ignores_termination_is_reported() -> None:
    balancer = await SpawnedBalancer.spawn()

    os.kill(balancer.process.pid, signal.SIGSTOP)

    with pytest.raises(ProcessDidNotExitError) as did_not_exit:
        await balancer.process.terminate()

    assert did_not_exit.value.pid == balancer.process.pid
