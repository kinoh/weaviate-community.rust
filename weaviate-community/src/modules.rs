use reqwest::Url;
use std::sync::Arc;
use crate::collections::error::{Result, WeaviateError};
use crate::collections::modules::{ContextionaryConcept, ContextionaryExtension};

/// All contextionary module related endpoints and functionality described in
/// [Weaviate contextionary API documentation](https://weaviate.io/developers/weaviate/modules/retriever-vectorizer-modules/text2vec-contextionary)
#[derive(Debug)]
pub struct Modules {
    endpoint: Url,
    client: Arc<reqwest::Client>,
}

impl Modules {
    /// Create a new Modules object. The modules object is intended to like inside the 
    /// WeaviateClient and be called through the WeaviateClient.
    pub(super) fn new(url: &Url, client: Arc<reqwest::Client>) -> Result<Self> {
        let endpoint = url.join("/v1/modules/")?;
        Ok(Modules { endpoint, client })
    }

    /// Get a concept from text2vec-contextionary.
    ///
    /// # Parameter
    /// - concept: the concept to search for
    ///
    /// # Example
    /// ```no_run
    /// use weaviate_community::WeaviateClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = WeaviateClient::builder("http://localhost:8080").build()?;
    ///     let res = client.modules.contextionary_get_concept("concept").await;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn contextionary_get_concept(
        &self,
        concept: &str
    ) -> Result<ContextionaryConcept> {
        let mut endpoint = String::from("text2vec-contextionary/concepts/");
        endpoint.push_str(concept);
        let endpoint = self.endpoint.join(&endpoint)?;
        let res = self.client.get(endpoint).send().await?;

        match res.status() {
            reqwest::StatusCode::OK => {
                let res: ContextionaryConcept = res.json().await?;
                Ok(res)
            },
            _ => Err(WeaviateError::from_response(
                WeaviateError::Module,
                "text2vec-contextionary concepts",
                res,
            ).await),
        }
    }

    /// Extend text2vec-contextionary.
    ///
    /// # Parameter
    /// - concept: the concept to extend contextionary with
    ///
    /// # Example
    /// ```no_run
    /// use weaviate_community::WeaviateClient;
    /// use weaviate_community::collections::modules::ContextionaryExtension;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = WeaviateClient::builder("http://localhost:8080").build()?;
    ///     let ext = ContextionaryExtension::new("concept", "description", 1.0);
    ///     let res = client.modules.contextionary_extend(ext).await;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn contextionary_extend(
        &self,
        concept: ContextionaryExtension
    ) -> Result<ContextionaryExtension> {
        let endpoint = self.endpoint.join("text2vec-contextionary/extensions")?;
        let res = self
            .client
            .post(endpoint)
            .json(&concept)
            .send()
            .await?;
        match res.status() {
            reqwest::StatusCode::OK => {
                let res: ContextionaryExtension = res.json().await?;
                Ok(res)
            },
            status => {
                let msg = match res.json::<serde_json::Value>().await {
                    Ok(json) => format!("status {status}, response: {json}"),
                    Err(_) => format!("status {status}, failed to parse response body"),
                };
                Err(WeaviateError::Module(msg))
            }
        }
    }

    
}

#[cfg(test)]
mod tests {
    use crate::{
        WeaviateClient,
        collections::modules::{
            ContextionaryExtension,
            ContextionaryConcept, IndividualWords
        }
    };

    async fn get_test_harness() -> (mockito::ServerGuard, WeaviateClient) {
        let mock_server = mockito::Server::new_async().await;
        let mut host = "http://".to_string();
        host.push_str(&mock_server.host_with_port());
        let client = WeaviateClient::builder(&host).build().unwrap();
        (mock_server, client)
    }

    fn get_mock_concept_response() -> String {
        serde_json::to_string(&ContextionaryConcept { 
            individual_words: vec![
                IndividualWords {
                    info: None,
                    word: "test".into(),
                    present: None,
                    concatenated_word: None,
                }
            ]
        }).unwrap()
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

    async fn mock_get(
        server: &mut mockito::ServerGuard,
        endpoint: &str,
        status_code: usize,
        body: &str,
    ) -> mockito::Mock {
        server
            .mock("GET", endpoint)
            .with_status(status_code)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create()
    }

    #[tokio::test]
    async fn test_get_concept_ok() {
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_get(
            &mut mock_server,
            "/v1/modules/text2vec-contextionary/concepts/test",
            200,
            &get_mock_concept_response(),
        ).await;
        let res = client.modules.contextionary_get_concept("test").await;
        mock.assert();
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_get_concept_err() {
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_get(
            &mut mock_server,
            "/v1/modules/text2vec-contextionary/concepts/test",
            401,
            "",
        ).await;
        let res = client.modules.contextionary_get_concept("test").await;
        mock.assert();
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_extend_ok() {
        let ext = ContextionaryExtension::new("test", "test", 1.0);
        let ext_str = serde_json::to_string(&ext).unwrap();
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_post(
            &mut mock_server,
            "/v1/modules/text2vec-contextionary/extensions",
            200,
            &ext_str,
        ).await;
        let res = client.modules.contextionary_extend(ext).await;
        mock.assert();
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_extend_err() {
        let (mut mock_server, client) = get_test_harness().await;
        let mock = mock_post(
            &mut mock_server,
            "/v1/modules/text2vec-contextionary/extensions",
            401,
            "",
        ).await;
        let res = client.modules.contextionary_extend(
            ContextionaryExtension::new("test", "test", 1.0)
        ).await;
        mock.assert();
        assert!(res.is_err());
    }
}
