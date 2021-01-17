use log::debug;
use log::error;
use log::info;
use reqwest::ClientBuilder;
use reqwest::Result;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use tame_oauth::gcp::prelude::*;

pub struct GCS;

#[derive(Serialize, Deserialize, Debug)]
struct TestIAMResponse {
    kind: String,
    permissions: Option<Vec<String>>,
}

impl GCS {
    fn base_url(&self, bucket: &str) -> String {
        format!("https://storage.googleapis.com/storage/v1/b/{}", bucket)
    }

    fn test_iam_url(&self, bucket: &str) -> String {
        self.base_url(bucket) + "/iam/testPermissions"
    }

    fn permissions(&self) -> &[(&str, &str)] {
        return &[
            ("permissions", "storage.buckets.delete"),
            ("permissions", "storage.buckets.get"),
            ("permissions", "storage.buckets.getIamPolicy"),
            ("permissions", "storage.buckets.setIamPolicy"),
            ("permissions", "storage.buckets.update"),
            ("permissions", "storage.objects.create"),
            ("permissions", "storage.objects.delete"),
            ("permissions", "storage.objects.get"),
            ("permissions", "storage.objects.list"),
            ("permissions", "storage.objects.update"),
        ];
    }

    pub async fn scan(&self, buckets: Vec<&str>) {
        for bucket in buckets {
            let exists = self.exists(bucket).await;
            if exists {
                debug!("Bucket {} exists", bucket);
                self.print_result(false, self.unauthenticated(bucket).await, bucket.clone());
                self.print_result(true, self.authenticated(bucket).await, bucket.clone());
            } else {
                error!("Bucket {} does not exist", bucket);
            }
        }
    }

    async fn authenticated(&self, bucket: &str) -> Result<Vec<String>> {
        let key_path =
            env::var("GOOGLE_APPLICATION_CREDENTIALS").expect("SET GOOGLE_APPLICATION_CREDENTIALS");
        let scopes: Vec<String> =
            vec!["https://www.googleapis.com/auth/devstorage.read_only".to_string()];
        let service_key = std::fs::read_to_string(key_path).expect("failed to read json key");
        let acct_info = ServiceAccountInfo::deserialize(service_key).unwrap();
        let acct_access = ServiceAccountAccess::new(acct_info).unwrap();

        let token = match acct_access.get_token(&scopes).unwrap() {
            TokenOrRequest::Request {
                request,
                scope_hash,
                ..
            } => {
                let client = reqwest::Client::new();

                let (parts, body) = request.into_parts();
                let uri = parts.uri.to_string();

                let builder = match parts.method {
                    http::Method::POST => client.post(&uri),
                    method => unimplemented!("{} not implemented", method),
                };

                let request = builder.headers(parts.headers).body(body).build().unwrap();
                let response = client.execute(request).await.unwrap();

                let mut builder = http::Response::builder()
                    .status(response.status())
                    .version(response.version());

                let headers = builder.headers_mut().unwrap();

                headers.extend(
                    response
                        .headers()
                        .into_iter()
                        .map(|(k, v)| (k.clone(), v.clone())),
                );

                let buffer = response.bytes().await.unwrap();
                let response = builder.body(buffer).unwrap();

                let token_response = acct_access.parse_token_response(scope_hash, response);
                if token_response.is_err() {
                    error!("{:?}", token_response.unwrap_err());
                    panic!();
                }

                token_response.unwrap()
            }
            _ => unreachable!(),
        };

        let request_url = self.test_iam_url(bucket);
        let timeout = Duration::new(5, 0);
        let query = self.permissions();

        let client = ClientBuilder::new().timeout(timeout).build()?;
        let response = client
            .get(&request_url)
            .header("Authorization", format!("Bearer {}", token.access_token))
            .query(query)
            .send()
            .await?;
        let json: TestIAMResponse = response.json().await?;

        return Ok(json.permissions.unwrap_or(Vec::new()));
    }

    async fn unauthenticated(&self, bucket: &str) -> Result<Vec<String>> {
        let request_url = self.test_iam_url(bucket);
        let timeout = Duration::new(5, 0);

        let client = ClientBuilder::new().timeout(timeout).build()?;
        let response = client
            .get(&request_url)
            .query(self.permissions())
            .send()
            .await?;

        if response.status().is_success() {
            let test_iam_response = response.json::<TestIAMResponse>().await?;
            return Ok(test_iam_response.permissions.unwrap_or(Vec::new()));
        }

        return Err(response.error_for_status().unwrap_err());
    }

    fn print_result(&self, authenticated: bool, permissions: Result<Vec<String>>, bucket: &str) {
        let role = if authenticated {
            "authenticated"
        } else {
            "unauthenticated"
        }
        .to_string();

        match permissions {
            Ok(permissions) => {
                if permissions.is_empty() {
                    info!("bucket={} has no allowed permissions as {}", bucket, role);
                }

                for permission in permissions {
                    info!("{}: {} - {:#?}", role, bucket, permission)
                }
            }
            Err(err) => error!("{:#?}", err),
        }
    }

    async fn exists(&self, bucket: &str) -> bool {
        let request_url = self.base_url(bucket);
        let timeout = Duration::new(5, 0);
        let client = ClientBuilder::new().timeout(timeout).build().unwrap();
        let response = client.head(&request_url).send().await.unwrap();

        response.status() != StatusCode::NOT_FOUND && response.status() != StatusCode::BAD_REQUEST
    }
}
