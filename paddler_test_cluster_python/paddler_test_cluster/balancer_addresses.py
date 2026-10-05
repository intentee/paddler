from pydantic import BaseModel, ConfigDict

from paddler_test_cluster.error import CompatOpenAIServiceNotServedError


class BalancerAddresses(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True)

    compat_openai: str | None
    inference: str
    management: str
    web_admin_panel: str | None

    @property
    def compat_openai_url(self) -> str:
        if self.compat_openai is None:
            raise CompatOpenAIServiceNotServedError

        return f"http://{self.compat_openai}"

    @property
    def inference_url(self) -> str:
        return f"http://{self.inference}"

    @property
    def management_url(self) -> str:
        return f"http://{self.management}"
