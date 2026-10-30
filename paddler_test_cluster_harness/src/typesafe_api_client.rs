use futures_util::TryFutureExt as _;
use reqwest::Client;
use reqwest::Error as ReqwestError;
use reqwest::RequestBuilder;
use serde_json::Value;
use url::Url;

use paddler_balancer::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use paddler_balancer::compatibility::typesafe_service::typesafe_header::TypeSafeHeader;

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

    pub async fn models(&self) -> Result<TypeSafeApiResponse, ReqwestError> {
        answered(self.client.get(self.endpoint(TypeSafeApiPath::MODELS))).await
    }

    pub async fn system_one(
        &self,
        request_id: &str,
        body: &Value,
    ) -> Result<TypeSafeApiResponse, ReqwestError> {
        answered(
            self.client
                .post(self.endpoint(TypeSafeApiPath::SYSTEM_ONE))
                .header(TypeSafeHeader::REQUEST_ID, request_id)
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
