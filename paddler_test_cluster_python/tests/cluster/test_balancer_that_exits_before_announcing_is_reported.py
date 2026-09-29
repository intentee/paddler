import pytest

from paddler_test_cluster.error import BalancerExitedBeforeAnnouncingError
from paddler_test_cluster.spawned_balancer import SpawnedBalancer

CLAP_USAGE_ERROR_STATUS = 2


async def test_balancer_that_exits_before_announcing_is_reported() -> None:
    with pytest.raises(BalancerExitedBeforeAnnouncingError) as exited_early:
        await SpawnedBalancer.spawn(state_database_url="file://relative/state.json")

    assert exited_early.value.returncode == CLAP_USAGE_ERROR_STATUS
