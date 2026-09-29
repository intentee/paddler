import asyncio
from dataclasses import dataclass
from datetime import timedelta

from paddler_test_cluster.balancer_addresses import BalancerAddresses
from paddler_test_cluster.error import BalancerExitedBeforeAnnouncingError
from paddler_test_cluster.observation_window import RELEASE
from paddler_test_cluster.paddler_binary_path import paddler_binary_path
from paddler_test_cluster.spawned_process import SpawnedProcess

EPHEMERAL_LOOPBACK_ADDRESS = "127.0.0.1:0"
MEMORY_STATE_DATABASE = "memory://"
PADDLER_BUFFERED_REQUEST_TIMEOUT = timedelta(seconds=10)


@dataclass(frozen=True)
class SpawnedBalancer:
    addresses: BalancerAddresses
    process: SpawnedProcess

    @classmethod
    async def spawn(
        cls,
        *,
        buffered_request_timeout: timedelta = PADDLER_BUFFERED_REQUEST_TIMEOUT,
        state_database_url: str = MEMORY_STATE_DATABASE,
    ) -> "SpawnedBalancer":
        process = await asyncio.create_subprocess_exec(
            paddler_binary_path(),
            "balancer",
            "--inference-addr",
            EPHEMERAL_LOOPBACK_ADDRESS,
            "--management-addr",
            EPHEMERAL_LOOPBACK_ADDRESS,
            "--compat-openai-addr",
            EPHEMERAL_LOOPBACK_ADDRESS,
            "--state-database",
            state_database_url,
            "--buffered-request-timeout",
            str(buffered_request_timeout // timedelta(milliseconds=1)),
            stdout=asyncio.subprocess.PIPE,
        )
        announcement = await process.stdout.readline() if process.stdout else b""

        if not announcement:
            async with asyncio.timeout(RELEASE.total_seconds()):
                raise BalancerExitedBeforeAnnouncingError(await process.wait())

        return cls(
            addresses=BalancerAddresses.model_validate_json(announcement),
            process=SpawnedProcess(process),
        )
