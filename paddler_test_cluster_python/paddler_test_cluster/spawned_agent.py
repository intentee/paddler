import asyncio

from paddler_test_cluster.agent_spec import AgentSpec
from paddler_test_cluster.paddler_binary_path import paddler_binary_path
from paddler_test_cluster.spawned_process import SpawnedProcess


async def spawn_agent(management_address: str, agent: AgentSpec) -> SpawnedProcess:
    process = await asyncio.create_subprocess_exec(
        paddler_binary_path(),
        "agent",
        "--management-addr",
        management_address,
        "--name",
        agent.name,
        "--slots",
        str(agent.slots),
        stdout=asyncio.subprocess.DEVNULL,
    )

    return SpawnedProcess(process)
