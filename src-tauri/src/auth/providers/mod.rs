use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::auth::token::TokenData;

pub mod twitch;

/// Authentication flow information returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationFlow {
    /// Device code flow (e.g., used by Twitch)
    DeviceCode {
        /// URL to visit to complete authentication
        verification_uri: String,
        /// Code to enter on the verification page
        user_code: String,
        /// Seconds until the flow expires
        expires_in: u64,
        /// Seconds to wait between polling attempts
        poll_interval: u64,
        /// Provider name
        provider: String,
    },
    
    /// Redirect flow (used for OAuth2 redirect flow)
    Redirect {
        /// URL to visit to complete authentication
        auth_url: String,
        /// Provider name
        provider: String,
    },
    
    /// Direct credentials (e.g., for testing or custom auth)
    Credentials {
        /// Provider name
        provider: String,
    },
}

/// Status of an authentication flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthFlowStatus {
    /// Flow is still pending user action
    Pending,
    /// Flow has completed successfully
    Completed {
        /// Token data from the completed flow
        token: TokenData,
    },
    /// Flow has expired
    Expired,
    /// Flow has failed
    Failed {
        /// Reason for failure
        reason: String,
    },
}

/// Common trait for all authentication providers
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;
    
    /// Start authentication flow
    async fn authenticate(&self) -> Result<AuthenticationFlow>;
    
    /// Refresh a token
    async fn refresh_token(&self, token: TokenData) -> Result<TokenData>;
    
    /// Validate a token
    async fn validate_token(&self, token: TokenData) -> Result<TokenData>;
    
    /// Check status of an authentication flow
    async fn check_flow_status(&self, flow_id: &str) -> Result<AuthFlowStatus>;
    
    /// Revoke a token (if supported)
    async fn revoke_token(&self, token: TokenData) -> Result<()> {
        // Default implementation does nothing
        Ok(())
    }
}

/// Store for authentication tokens
pub struct TokenStore {
    tokens: Arc<RwLock<HashMap<String, TokenData>>>,
}

impl TokenStore {
    /// Create a new token store
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Store a token
    pub async fn store(&self, provider: &str, token: &TokenData) -> Result<()> {
        self.tokens.write().await.insert(provider.to_string(), token.clone());
        Ok(())
    }
    
    /// Retrieve a token
    pub async fn get(&self, provider: &str) -> Result<Option<TokenData>> {
        Ok(self.tokens.read().await.get(provider).cloned())
    }
    
    /// Remove a token
    pub async fn remove(&self, provider: &str) -> Result<()> {
        self.tokens.write().await.remove(provider);
        Ok(())
    }
    
    /// Clear all tokens
    pub async fn clear(&self) -> Result<()> {
        self.tokens.write().await.clear();
        Ok(())
    }
}

/// Record of a scheduled token refresh
#[derive(Debug, Clone)]
pub struct ScheduledRefresh {
    /// Provider to refresh
    pub provider: String,
    /// When to perform the refresh
    pub refresh_time: DateTime<Utc>,
}

impl ScheduledRefresh {
    /// Create a new scheduled refresh
    pub fn new(provider: &str, refresh_time: DateTime<Utc>) -> Self {
        Self {
            provider: provider.to_string(),
            refresh_time,
        }
    }
    
    /// Check if the refresh should be performed now
    pub fn is_due(&self) -> bool {
        self.refresh_time <= Utc::now()
    }
}