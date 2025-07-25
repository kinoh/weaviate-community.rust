use reqwest::Url;
use std::sync::Arc;

use crate::collections::backups::{
    BackupBackends, BackupCreateRequest, BackupResponse, BackupRestoreRequest, BackupStatus,
    BackupStatusResponse,
};
use crate::collections::error::{Result, WeaviateError};

/// All backup related endpoints and functionality described in
/// [Weaviate meta API documentation](https://weaviate.io/developers/weaviate/api/rest/backups)
#[derive(Debug)]
pub struct Backups {
    endpoint: Url,
    client: Arc<reqwest::Client>,
}

impl Backups {
    pub(super) fn new(url: &Url, client: Arc<reqwest::Client>) -> Result<Self> {
        let endpoint = url.join("/v1/backups/")?;
        Ok(Backups { endpoint, client })
    }

    /// Create a new backup
    ///
    /// # Examples
    /// Creating a backup to the filesystem, waiting for completion
    /// ```no_run
    /// use weaviate_community::WeaviateClient;
    /// use weaviate_community::collections::backups::{BackupBackends, BackupCreateRequest};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = WeaviateClient::builder("http://localhost:8080").build()?;
    ///     let my_request = BackupCreateRequest {
    ///         id: "doc-test-backup".into(),
    ///         include: None,
    ///         exclude: None
    ///     };
    ///     let res = client.backups.create(
    ///         &BackupBackends::FILESYSTEM,
    ///         &my_request,
    ///         true
    ///     ).await?;
    ///     println!("{:#?}", res);
    ///     Ok(())
    /// }
    /// ```
    pub async fn create(
        &self,
        backend: &BackupBackends,
        backup_request: &BackupCreateRequest,
        wait_for_completion: bool,
    ) -> Result<BackupResponse> {
        let endpoint = self.endpoint.join(backend.value())?;
        let payload = serde_json::to_value(&backup_request)?;
        let res = self.client.post(endpoint).json(&payload).send().await?;

        match res.status() {
            reqwest::StatusCode::OK => {
                let mut res: BackupResponse = res.json().await?;
                if wait_for_completion {
                    let complete = self
                        .wait_for_completion(&backend, &backup_request.id, false)
                        .await?;
                    res.status = complete;
                }
                Ok(res)
            }
            _ => Err(WeaviateError::Backup(format!(
                "status code {} received.",
                res.status()
            ))),
        }
    }

    /// Get the status of a backup
    ///
    /// # Examples
    /// ```no_run
    /// use weaviate_community::WeaviateClient;
    /// use weaviate_community::collections::backups::BackupBackends;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = WeaviateClient::builder("http://localhost:8080").build()?;
    ///     let res = client.backups.get_backup_status(
    ///         &BackupBackends::FILESYSTEM,
    ///         "doc-test-backup",
    ///         true
    ///     ).await?;
    ///     println!("{:#?}", res);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_backup_status(
        &self,
        backend: &BackupBackends,
        backup_id: &str,
        restore: bool,
    ) -> Result<BackupStatusResponse> {
        let mut endpoint: String = backend.value().into();
        endpoint.push_str("/");
        endpoint.push_str(&backup_id.to_string());
        if restore {
            endpoint.push_str("/restore");
        }
        let endpoint = self.endpoint.join(&endpoint)?;
        let res = self.client.get(endpoint).send().await?;
        match res.status() {
            reqwest::StatusCode::OK => {
                let res: BackupStatusResponse = res.json().await?;
                Ok(res)
            }
            _ => Err(WeaviateError::Backup(format!(
                "status code {} received.",
                res.status()
            ))),
        }
    }

    /// Restore a backup
    ///
    /// # Examples
    /// Restore a backup from the filesystem, waiting for completion
    /// ```no_run
    /// use weaviate_community::WeaviateClient;
    /// use weaviate_community::collections::backups::{BackupBackends, BackupRestoreRequest};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = WeaviateClient::builder("http://localhost:8080").build()?;
    ///     let my_request = BackupRestoreRequest {
    ///         include: None,
    ///         exclude: None
    ///     };
    ///     let res = client.backups.restore(
    ///         &BackupBackends::FILESYSTEM,
    ///         "doc-test-backup",
    ///         &my_request,
    ///         true
    ///     ).await?;
    ///     println!("{:#?}", res);
    ///     Ok(())
    /// }
    /// ```
    pub async fn restore(
        &self,
        backend: &BackupBackends,
        backup_id: &str,
        backup_request: &BackupRestoreRequest,
        wait_for_completion: bool,
    ) -> Result<BackupResponse> {
        let mut endpoint: String = backend.value().into();
        endpoint.push_str("/");
        endpoint.push_str(&backup_id.to_string());
        endpoint.push_str("/restore");
        let endpoint = self.endpoint.join(&endpoint)?;
        let payload = serde_json::to_value(&backup_request)?;
        let res = self.client.post(endpoint).json(&payload).send().await?;

        match res.status() {
            reqwest::StatusCode::OK => {
                let mut res: BackupResponse = res.json().await?;
                if wait_for_completion {
                    let complete = self.wait_for_completion(&backend, &backup_id, true).await?;
                    res.status = complete;
                }
                Ok(res)
            }
            _ => Err(WeaviateError::Backup(format!(
                "status code {} received.",
                res.status()
            ))),
        }
    }

    /// Wait for a backup to complete before returning
    async fn wait_for_completion(
        &self,
        backend: &BackupBackends,
        backup_id: &str,
        restore: bool,
    ) -> Result<BackupStatus> {
        loop {
            let res = self.get_backup_status(backend, backup_id, restore).await;
            let status = res?;
            if status.status == BackupStatus::SUCCESS {
                return Ok(BackupStatus::SUCCESS);
            } else if status.status == BackupStatus::FAILED {
                return Err(WeaviateError::Backup(format!("backup status FAILED",)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        collections::backups::{
            BackupBackends, BackupCreateRequest, BackupResponse, BackupRestoreRequest,
            BackupStatus, BackupStatusResponse,
        },
        WeaviateClient,
    };

    async fn get_test_harness() -> (mockito::ServerGuard, WeaviateClient) {
        let mock_server = mockito::Server::new_async().await;
        let mut host = "http://".to_string();
        host.push_str(&mock_server.host_with_port());
        let client = WeaviateClient::builder(&host).build().unwrap();
        (mock_server, client)
    }

    fn test_create_backup_request() -> BackupCreateRequest {
        BackupCreateRequest::builder("abcd").build()
    }

    fn test_restore_backup_request() -> BackupRestoreRequest {
        BackupRestoreRequest::builder().build()
    }

    fn test_backup_response(status: BackupStatus) -> BackupResponse {
        BackupResponse {
            id: "abcd".into(),
            classes: Vec::new(),
            path: "".into(),
            backend: BackupBackends::FILESYSTEM,
            status,
        }
    }

    fn test_backup_status(status: BackupStatus) -> BackupStatusResponse {
        BackupStatusResponse {
            id: "abcd".into(),
            path: None,
            backend: BackupBackends::FILESYSTEM.value().into(),
            status,
        }
    }

    async fn mock_get(
        server: &mut mockito::ServerGuard,
        endpoint: &str,
        status_code: usize,
        body: &str,
    ) -> mockito::Mock {
        server
            .mock("GET", endpoint)
            .with_status(status_code)
            .with_body(body)
            .create()
    }

    async fn mock_post(
        server: &mut mockito::ServerGuard,
        endpoint: &str,
        status_code: usize,
        body: &str,
    ) -> mockito::Mock {
        server
            .mock("POST", endpoint)
            .with_status(status_code)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create()
    }

    #[tokio::test]
    async fn test_get_backup_status_ok() {
        let out = test_backup_status(BackupStatus::SUCCESS);
        let out_str = serde_json::to_string(&out).unwrap();
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_get(
            &mut mock_server,
            "/v1/backups/filesystem/abcd",
            200,
            &out_str,
        ).await;
        let res = client
            .backups
            .get_backup_status(&BackupBackends::FILESYSTEM, "abcd", false)
            .await;
        mock.assert();
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_get_backup_status_err() {
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_get(&mut mock_server, "/v1/backups/filesystem/abcd", 404, "").await;
        let res = client
            .backups
            .get_backup_status(&BackupBackends::FILESYSTEM, "abcd", false)
            .await;
        mock.assert();
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_create_backup_ok() {
        let req = test_create_backup_request();
        let out = test_backup_response(BackupStatus::STARTED);
        let out_str = serde_json::to_string(&out).unwrap();
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_post(&mut mock_server, "/v1/backups/filesystem", 200, &out_str).await;
        let res = client
            .backups
            .create(&BackupBackends::FILESYSTEM, &req, false)
            .await;
        mock.assert();
        assert!(res.is_ok());
        assert_eq!(req.id, res.unwrap().id);
    }

    #[tokio::test]
    async fn test_create_backup_err() {
        let req = test_create_backup_request();
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_post(&mut mock_server, "/v1/backups/filesystem", 404, "").await;
        let res = client
            .backups
            .create(&BackupBackends::FILESYSTEM, &req, false)
            .await;
        mock.assert();
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_create_backup_wait_ok() {
        let req = test_create_backup_request();
        let out = test_backup_response(BackupStatus::STARTED);
        let out_str = serde_json::to_string(&out).unwrap();
        let out_two = test_backup_status(BackupStatus::SUCCESS);
        let out_two_str = serde_json::to_string(&out_two).unwrap();
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_post(&mut mock_server, "/v1/backups/filesystem", 200, &out_str).await;
        let mock2 = mock_get(
            &mut mock_server,
            "/v1/backups/filesystem/abcd",
            200,
            &out_two_str,
        ).await;
        let res = client
            .backups
            .create(&BackupBackends::FILESYSTEM, &req, true)
            .await;
        mock.assert();
        mock2.assert();
        assert!(res.is_ok());
        assert_eq!(BackupStatus::SUCCESS, res.unwrap().status);
    }

    #[tokio::test]
    async fn test_create_backup_wait_err() {
        let req = test_create_backup_request();
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_post(&mut mock_server, "/v1/backups/filesystem", 404, "").await;
        let res = client
            .backups
            .create(&BackupBackends::FILESYSTEM, &req, true)
            .await;
        mock.assert();
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_restore_backup_ok() {
        let req = test_restore_backup_request();
        let out = test_backup_response(BackupStatus::STARTED);
        let out_str = serde_json::to_string(&out).unwrap();
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_post(
            &mut mock_server,
            "/v1/backups/filesystem/abcd/restore",
            200,
            &out_str,
        ).await;
        let res = client
            .backups
            .restore(&BackupBackends::FILESYSTEM, "abcd", &req, false)
            .await;
        mock.assert();
        assert!(res.is_ok());
        assert_eq!(BackupStatus::STARTED, res.unwrap().status);
    }

    #[tokio::test]
    async fn test_restore_backup_err() {
        let req = test_restore_backup_request();
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_post(
            &mut mock_server,
            "/v1/backups/filesystem/abcd/restore",
            404,
            "",
        ).await;
        let res = client
            .backups
            .restore(&BackupBackends::FILESYSTEM, "abcd", &req, false)
            .await;
        mock.assert();
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_restore_backup_wait_ok() {
        let req = test_restore_backup_request();
        let out = test_backup_response(BackupStatus::STARTED);
        let out_str = serde_json::to_string(&out).unwrap();
        let out_two = test_backup_status(BackupStatus::SUCCESS);
        let out_two_str = serde_json::to_string(&out_two).unwrap();
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_post(
            &mut mock_server,
            "/v1/backups/filesystem/abcd/restore",
            200,
            &out_str,
        ).await;
        let mock2 = mock_get(
            &mut mock_server,
            "/v1/backups/filesystem/abcd/restore",
            200,
            &out_two_str,
        ).await;
        let res = client
            .backups
            .restore(&BackupBackends::FILESYSTEM, "abcd", &req, true)
            .await;
        mock.assert();
        mock2.assert();
        assert!(res.is_ok());
        assert_eq!(BackupStatus::SUCCESS, res.unwrap().status);
    }

    #[tokio::test]
    async fn test_restore_backup_wait_err() {
        let req = test_restore_backup_request();
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_post(
            &mut mock_server,
            "/v1/backups/filesystem/abcd/restore",
            404,
            "",
        ).await;
        let res = client
            .backups
            .restore(&BackupBackends::FILESYSTEM, "abcd", &req, true)
            .await;
        mock.assert();
        assert!(res.is_err());
    }
}
