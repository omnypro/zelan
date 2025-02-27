use anyhow::Result;
use reqwest;
use std::collections::HashMap;

/// HTTP method enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    GET,
    POST,
}

/// A very simple version that only holds response data
#[derive(Debug, Clone)]
pub struct SimpleHttpResponse {
    /// HTTP status code
    status_code: u16,
    /// Response body
    body: String,
    /// Response headers
    headers: HashMap<String, String>,
}

impl SimpleHttpResponse {
    /// Create a new response
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status_code: status,
            body: body.into(),
            headers: HashMap::new(),
        }
    }

    /// Add a header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Get the status code
    pub fn status(&self) -> u16 {
        self.status_code
    }

    /// Get a reference to the response body
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Get the body as text (consumes the response)
    pub fn text(self) -> String {
        self.body
    }

    /// Parse body as JSON
    pub fn json<T: serde::de::DeserializeOwned>(self) -> Result<T> {
        Ok(serde_json::from_str(&self.body)?)
    }

    /// Check if successful (2xx status)
    pub fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }
}

/// Trait for HTTP client operations, allowing for mocking
#[async_trait::async_trait]
pub trait HttpClient: Send + Sync {
    /// Perform HTTP GET request and return a SimpleHttpResponse
    async fn get(&self, url: &str, headers: HashMap<String, String>) -> Result<SimpleHttpResponse>;

    /// Perform HTTP POST request and return a SimpleHttpResponse
    async fn post(
        &self,
        url: &str,
        headers: HashMap<String, String>,
        body: String,
    ) -> Result<SimpleHttpResponse>;
}

/// Implementation of HttpClient using reqwest
pub struct ReqwestHttpClient {
    /// Internal reqwest client
    client: reqwest::Client,
}

impl ReqwestHttpClient {
    /// Create a new ReqwestHttpClient
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Create a new client with custom configuration
    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn get(&self, url: &str, headers: HashMap<String, String>) -> Result<SimpleHttpResponse> {
        let mut request = self.client.get(url);

        // Add headers
        for (key, value) in headers {
            request = request.header(key, value);
        }

        // Send request
        let response = request.send().await?;

        // Convert to SimpleHttpResponse
        let status = response.status().as_u16();
        let body = response.text().await?;

        // Build our response with headers
        let result = SimpleHttpResponse::new(status, body);

        // We could extract headers here if needed
        // response.headers().iter().for_each(|(k, v)| {
        //    if let Ok(value_str) = v.to_str() {
        //        result = result.with_header(k.as_str(), value_str);
        //    }
        // });

        Ok(result)
    }

    async fn post(
        &self,
        url: &str,
        headers: HashMap<String, String>,
        body: String,
    ) -> Result<SimpleHttpResponse> {
        let mut request = self.client.post(url).body(body);

        // Add headers
        for (key, value) in headers {
            request = request.header(key, value);
        }

        // Send request
        let response = request.send().await?;

        // Convert to SimpleHttpResponse
        let status = response.status().as_u16();
        let body = response.text().await?;

        Ok(SimpleHttpResponse::new(status, body))
    }
}

/// Mock implementation of HttpClient for testing
#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A mock HTTP client that returns predefined responses
    pub struct MockHttpClient {
        /// Map of URLs to responses
        responses: Arc<Mutex<HashMap<String, SimpleHttpResponse>>>,
        /// Record of requests made (URL, method)
        requests: Arc<Mutex<Vec<(String, HttpMethod)>>>,
    }

    impl MockHttpClient {
        /// Create a new mock client
        pub fn new() -> Self {
            Self {
                responses: Arc::new(Mutex::new(HashMap::new())),
                requests: Arc::new(Mutex::new(Vec::new())),
            }
        }

        /// Register a mock response for a URL
        pub fn mock_response(
            &mut self,
            url: impl Into<String>,
            status: u16,
            body: impl Into<String>,
        ) {
            let response = SimpleHttpResponse::new(status, body);
            self.responses.lock().unwrap().insert(url.into(), response);
        }

        /// Register a JSON response
        pub fn mock_json<T: serde::Serialize>(
            &mut self,
            url: impl Into<String>,
            status: u16,
            data: &T,
        ) -> Result<()> {
            let body = serde_json::to_string(data)?;

            let response = SimpleHttpResponse::new(status, body)
                .with_header("content-type", "application/json");

            self.responses.lock().unwrap().insert(url.into(), response);
            Ok(())
        }

        /// Update an existing mock response with a different message
        pub fn update_mock_response(
            &mut self,
            url: impl Into<String>,
            status: u16,
            body: impl Into<String>,
        ) {
            let response = SimpleHttpResponse::new(status, body);
            self.responses.lock().unwrap().insert(url.into(), response);
        }

        /// Mock a successful JSON response (status 200)
        pub fn mock_success_json<T: serde::Serialize>(
            &mut self,
            url: impl Into<String>,
            data: &T,
        ) -> Result<()> {
            self.mock_json(url, 200, data)
        }

        /// Mock an error response
        pub fn mock_error(
            &mut self,
            url: impl Into<String>,
            status: u16,
            message: impl Into<String>,
        ) {
            self.mock_response(url, status, message);
        }

        /// Get the list of recorded requests
        pub fn get_requests(&self) -> Vec<(String, HttpMethod)> {
            self.requests.lock().unwrap().clone()
        }

        /// Record a request
        fn record_request(&self, url: String, method: HttpMethod) {
            self.requests.lock().unwrap().push((url, method));
        }

        /// Get mock response for URL (cloned)
        fn get_response_for(&self, url: &str) -> Result<SimpleHttpResponse> {
            let responses = self.responses.lock().unwrap();
            responses
                .get(url)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No mock response configured for URL: {}", url))
        }
    }

    #[async_trait::async_trait]
    impl HttpClient for MockHttpClient {
        async fn get(
            &self,
            url: &str,
            _headers: HashMap<String, String>,
        ) -> Result<SimpleHttpResponse> {
            // Record the request
            self.record_request(url.to_string(), HttpMethod::GET);

            // Return the configured response
            self.get_response_for(url)
        }

        async fn post(
            &self,
            url: &str,
            _headers: HashMap<String, String>,
            _body: String,
        ) -> Result<SimpleHttpResponse> {
            // Record the request
            self.record_request(url.to_string(), HttpMethod::POST);

            // Return the configured response
            self.get_response_for(url)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_mock_http_client() -> Result<()> {
        use mock::MockHttpClient;

        // Setup
        let mut client = MockHttpClient::new();

        // Configure mock responses
        client.mock_response("https://example.com/api", 200, "Hello, world!");

        // JSON response example
        let test_data = serde_json::json!({
            "data": [{"id": "123", "name": "Test"}]
        });
        client.mock_success_json("https://example.com/api/json", &test_data)?;

        // Error response
        client.mock_error("https://example.com/api/error", 404, "Not found");

        // Test GET text response
        let headers = HashMap::new();
        let response = client.get("https://example.com/api", headers).await?;
        assert_eq!(response.status(), 200);
        assert_eq!(response.text(), "Hello, world!");

        // Test GET JSON response
        let headers = HashMap::new();
        let response = client.get("https://example.com/api/json", headers).await?;
        assert_eq!(response.status(), 200);
        let json: serde_json::Value = response.json()?;
        assert_eq!(json, test_data);

        // Test error response
        let headers = HashMap::new();
        let response = client.get("https://example.com/api/error", headers).await?;
        assert_eq!(response.status(), 404);
        assert_eq!(response.text(), "Not found");

        // Test missing URL
        let headers = HashMap::new();
        let result = client.get("https://example.com/not-found", headers).await;
        assert!(result.is_err());

        // Check request recording
        let requests = client.get_requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[0].0, "https://example.com/api");
        assert!(matches!(requests[0].1, HttpMethod::GET));

        Ok(())
    }
}
