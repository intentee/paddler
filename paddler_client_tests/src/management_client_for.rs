use paddler_client::management_client::ManagementClient;
use paddler_client::management_client_params::ManagementClientParams;
use url::Url;

#[must_use]
pub fn management_client_for(url: Url) -> ManagementClient {
    ManagementClient::new(ManagementClientParams { url })
}
