use futures_util::TryFutureExt as _;
use reqwest::Client;
use reqwest::Error as ReqwestError;
use reqwest::RequestBuilder;
use serde_json::Value;
use url::Url;

use paddler_balancer::constants::TYPESAFE_API_PATH_MODELS;
use paddler_balancer::constants::TYPESAFE_API_PATH_SYSTEM_ONE;
use paddler_balancer::constants::TYPESAFE_HEADER_REQUEST_ID;

use crate::typesafe_api_response::TypeSafeApiResponse;

fn answered(
    request: RequestBuilder,
) -> impl Future<Output = Result<TypeSafeApiResponse, ReqwestError>> {
    request.send().and_then(|response| {
        let headers = response.headers().clone();
        let status = response.status();

        response.json().map_ok(move |body| TypeSafeApiResponse {
            body,
            headers,
            status,
        })
    })
}

#[derive(Clone)]
pub struct TypeSafeApiClient {
    client: Client,
    typesafe_base_url: Url,
}

impl TypeSafeApiClient {
    #[must_use]
    pub fn new(typesafe_base_url: Url) -> Self {
        Self {
            client: Client::new(),
            typesafe_base_url,
        }
    }

    /// # Errors
    ///
    /// Returns the [`ReqwestError`] of a models request that fails or whose body is not JSON.
    pub async fn models(&self) -> Result<TypeSafeApiResponse, ReqwestError> {
        answered(self.client.get(self.endpoint(TYPESAFE_API_PATH_MODELS))).await
    }

    /// # Errors
    ///
    /// Returns the [`ReqwestError`] of a system one request that fails or whose body is not JSON.
    pub async fn system_one(
        &self,
        request_id: &str,
        body: &Value,
    ) -> Result<TypeSafeApiResponse, ReqwestError> {
        answered(
            self.client
                .post(self.endpoint(TYPESAFE_API_PATH_SYSTEM_ONE))
                .header(TYPESAFE_HEADER_REQUEST_ID, request_id)
                .json(body),
        )
        .await
    }

    fn endpoint(&self, path: &str) -> Url {
        let mut endpoint = self.typesafe_base_url.clone();

        endpoint.set_path(path);

        endpoint
    }
}
