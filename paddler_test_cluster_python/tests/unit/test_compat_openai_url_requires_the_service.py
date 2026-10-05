import pytest

from paddler_test_cluster.balancer_addresses import BalancerAddresses
from paddler_test_cluster.error import CompatOpenAIServiceNotServedError


def test_compat_openai_url_requires_the_service() -> None:
    addresses = BalancerAddresses(
        compat_openai=None,
        inference="127.0.0.1:8061",
        management="127.0.0.1:8060",
        web_admin_panel=None,
    )

    with pytest.raises(CompatOpenAIServiceNotServedError):
        _ = addresses.compat_openai_url
