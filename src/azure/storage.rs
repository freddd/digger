use log::{debug, error, info};
use reqwest::ClientBuilder;
use std::{error::Error, time::Duration};

#[derive(Debug, Clone)]
pub struct AzureStorage {
    account: String,
}

impl AzureStorage {
    fn base_url_with_container(&self, container: &str) -> String {
        format!(
            "https://{}.blob.core.windows.net/{}",
            self.account, container
        )
    }

    fn timeout(&self) -> Duration {
        Duration::new(5, 0)
    }

    pub fn new(account: &str) -> AzureStorage {
        AzureStorage {
            account: String::from(account),
        }
    }
    pub async fn scan(&self, containers: Vec<&str>) {
        for container in containers {
            match self.exists(container).await {
                Ok(_) => {
                    debug!("container={}: exists", container);
                    let list = self.list(container).await;
                    if list.is_ok() {
                        info!("container={}: successfully listed objects", container);
                    }
                }
                Err(_) => {
                    error!("container={}: does not exist", container);
                }
            }
        }
    }

    async fn exists(&self, container: &str) -> Result<(), Box<dyn Error>> {
        let url = self.base_url_with_container(container);
        let client = ClientBuilder::new().timeout(self.timeout()).build()?;
        let response = client
            .get(&url)
            .query(&[("restype", "container")])
            .send()
            .await?;

        if response.status().is_success() {
            return Ok(());
        }

        Err(Box::new(response.error_for_status().unwrap_err()))
    }
    async fn list(&self, container: &str) -> Result<(), Box<dyn Error>> {
        let url = self.base_url_with_container(container);
        let client = ClientBuilder::new().timeout(self.timeout()).build()?;
        let response = client.get(&url).query(&[("comp", "list")]).send().await?;

        if response.status().is_success() {
            return Ok(());
        }

        Err(Box::new(response.error_for_status().unwrap_err()))
    }
    async fn _upload(&self, _container: &str) {
        panic!("Not yet implemented")
    }
    async fn _delete(&self, _container: &str) {
        panic!("Not yet implemented")
    }
}
