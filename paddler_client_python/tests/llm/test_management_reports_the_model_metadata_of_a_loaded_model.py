from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_management import ClientManagement


async def test_management_reports_the_model_metadata_of_a_loaded_model(
    qwen3_cluster: BalancerAddresses,
) -> None:
    async with ClientManagement(url=qwen3_cluster.management_url) as client:
        [agent] = (await client.get_agents()).agents
        model_metadata = await client.get_model_metadata(agent.id)

    assert model_metadata is not None
    assert model_metadata.metadata["general.architecture"] == "qwen3"
