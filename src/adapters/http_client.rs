//! HTTP client abstraction for adapters
//!
//! This module provides a clean interface for making HTTP requests,
//! which can be easily mocked for testing.

use anyhow::Result;
use async_trait::async_trait;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Method, StatusCode,
};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;

/// Simple HTTP response structure for standardized response handling
#[derive(Debug, Clone)]
pub struct SimpleHttpResponse {
    /// HTTP status code
    pub status: StatusCode,
    /// Response headers
    pub headers: HeaderMap,
    /// Response body as text
    pub body: String,
}

impl SimpleHttpResponse {
    /// Parse the response body as JSON
    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_str(&self.body)?)
    }

    /// Check if the response is successful (status code 200-299)
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Check if the response is a client error (status code 400-499)
    pub fn is_client_error(&self) -> bool {
        self.status.is_client_error()
    }

    /// Check if the response is a server error (status code 500-599)
    pub fn is_server_error(&self) -> bool {
        self.status.is_server_error()
    }
}

/// HTTP client trait for abstracting HTTP requests
#[async_trait]
pub trait HttpClient: Send + Sync + Debug {
    /// Send an HTTP request with the specified method, URL, headers, and body
    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    ) -> Result<SimpleHttpResponse>;

    /// Send a GET request
    async fn get(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> Result<SimpleHttpResponse> {
        self.request("GET", url, headers, None).await
    }

    /// Send a POST request
    async fn post(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    ) -> Result<SimpleHttpResponse> {
        self.request("POST", url, headers, body).await
    }

    /// Send a PUT request
    async fn put(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    ) -> Result<SimpleHttpResponse> {
        self.request("PUT", url, headers, body).await
    }

    /// Send a DELETE request
    async fn delete(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> Result<SimpleHttpResponse> {
        self.request("DELETE", url, headers, None).await
    }

    /// Send a PATCH request
    async fn patch(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    ) -> Result<SimpleHttpResponse> {
        self.request("PATCH", url, headers, body).await
    }
}

/// Implementation of HttpClient using reqwest
#[derive(Debug, Clone)]
pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl ReqwestHttpClient {
    /// Create a new ReqwestHttpClient
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    ) -> Result<SimpleHttpResponse> {
        // Convert method string to reqwest Method
        let method = Method::from_str(method.to_uppercase().as_str())?;

        // Build request with method and URL
        let mut request_builder = self.client.request(method, url);

        // Add headers if provided
        if let Some(headers) = headers {
            let mut header_map = HeaderMap::new();
            for (key, value) in headers {
                let header_name = HeaderName::from_str(&key)?;
                let header_value = HeaderValue::from_str(&value)?;
                header_map.insert(header_name, header_value);
            }
            request_builder = request_builder.headers(header_map);
        }

        // Add body if provided
        if let Some(body) = body {
            request_builder = request_builder.body(body);
        }

        // Send request and get response
        let response = request_builder.send().await?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await?;

        Ok(SimpleHttpResponse {
            status,
            headers,
            body,
        })
    }
}

/// Mock HTTP client for testing
#[derive(Debug, Clone)]
pub struct MockHttpClient {
    /// Map of request URL to response
    responses: Arc<parking_lot::RwLock<HashMap<String, SimpleHttpResponse>>>,
}

impl MockHttpClient {
    /// Create a new MockHttpClient
    pub fn new() -> Self {
        Self {
            responses: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    /// Add a mock response for a URL
    pub fn add_response(&self, url: &str, response: SimpleHttpResponse) {
        self.responses
            .write()
            .insert(url.to_string(), response);
    }

    /// Add a mock JSON response for a URL
    pub fn add_json_response<T: serde::Serialize>(
        &self,
        url: &str,
        status: StatusCode,
        data: &T,
    ) -> Result<()> {
        let body = serde_json::to_string(data)?;
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        );

        self.add_response(
            url,
            SimpleHttpResponse {
                status,
                headers,
                body,
            },
        );

        Ok(())
    }

    /// Clear all mock responses
    pub fn clear(&self) {
        self.responses.write().clear();
    }
}

#[async_trait]
impl HttpClient for MockHttpClient {
    async fn request(
        &self,
        _method: &str,
        url: &str,
        _headers: Option<HashMap<String, String>>,
        _body: Option<String>,
    ) -> Result<SimpleHttpResponse> {
        // Look up the response for this URL
        let responses = self.responses.read();
        match responses.get(url) {
            Some(response) => Ok(response.clone()),
            None => Err(anyhow::anyhow!("No mock response for URL: {}", url)),
        }
    }
}

impl Default for MockHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_mock_http_client() {
        let client = MockHttpClient::new();
        
        // Add a mock response
        let mock_response = SimpleHttpResponse {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: r#"{"name":"test"}"#.to_string(),
        };
        client.add_response("https://example.com/api", mock_response);
        
        // Test the response
        let response = client.get("https://example.com/api", None).await.unwrap();
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body, r#"{"name":"test"}"#);
        
        let data: serde_json::Value = response.json().unwrap();
        assert_eq!(data["name"], "test");
    }

    #[tokio::test]
    async fn test_add_json_response() {
        let client = MockHttpClient::new();
        
        // Add a JSON response
        let data = json!({"id": 123, "name": "Test"});
        client.add_json_response("https://example.com/api/json", StatusCode::OK, &data).unwrap();
        
        // Test the response
        let response = client.get("https://example.com/api/json", None).await.unwrap();
        assert_eq!(response.status, StatusCode::OK);
        
        let result: serde_json::Value = response.json().unwrap();
        assert_eq!(result["id"], 123);
        assert_eq!(result["name"], "Test");
    }

    // The ReqwestHttpClient tests would require network connectivity,
    // so we'll skip actual implementation but note they should test:
    // - Basic GET requests
    // - POST with JSON body
    // - Setting custom headers
    // - Handling different status codes
}