use std::collections::HashMap;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::{Result, anyhow};

/// Token data structure that represents authentication credentials for a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    /// The access token used for API requests
    pub access_token: String,
    
    /// An optional refresh token used to obtain new access tokens
    pub refresh_token: Option<String>,
    
    /// When the access token expires (if known)
    pub expiration: Option<DateTime<Utc>>,
    
    /// The authentication provider (e.g., "twitch")
    pub provider: String,
    
    /// The permissions granted to this token
    pub scope: Vec<String>,
    
    /// The user identifier associated with this token
    pub user_id: Option<String>,
    
    /// Additional provider-specific information
    pub metadata: HashMap<String, Value>,
    
    /// When this token was created or refreshed
    pub created_at: DateTime<Utc>,
}

impl TokenData {
    /// Create a new token data structure
    pub fn new(
        access_token: String,
        refresh_token: Option<String>,
        expiration: Option<DateTime<Utc>>,
        provider: String,
        scope: Vec<String>,
        user_id: Option<String>,
    ) -> Self {
        Self {
            access_token,
            refresh_token,
            expiration,
            provider,
            scope,
            user_id,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }
    
    /// Check if the token is valid (not expired)
    pub fn is_valid(&self) -> bool {
        !self.access_token.is_empty() && !self.is_expired()
    }
    
    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        self.expiration.map_or(false, |exp| exp <= Utc::now())
    }
    
    /// Check if the token will expire soon (within the given threshold)
    pub fn expires_soon(&self, threshold_secs: i64) -> bool {
        self.expiration.map_or(false, |exp| {
            (exp - Utc::now()).num_seconds() < threshold_secs
        })
    }
    
    /// Check if the token should be proactively refreshed (75% of lifetime elapsed)
    pub fn should_proactively_refresh(&self) -> bool {
        if let (Some(exp), Some(created)) = (self.expiration, Some(self.created_at)) {
            let total_lifetime = exp - created;
            let elapsed = Utc::now() - created;
            elapsed > total_lifetime * 0.75
        } else {
            // If we don't know when it expires, be conservative and refresh
            // after one hour to be safe
            let one_hour = Duration::hours(1);
            Utc::now() - self.created_at > one_hour
        }
    }
    
    /// Check if the refresh token is approaching its expiration
    /// Twitch refresh tokens expire after 30 days of inactivity
    pub fn refresh_token_expiring_soon(&self, days_threshold: i64) -> bool {
        if self.refresh_token.is_none() {
            return false;
        }
        
        // Time since token creation (or last refresh)
        let days_since_created = (Utc::now() - self.created_at).num_days();
        
        // Twitch refresh tokens expire after 30 days, so warn when approaching that limit
        days_since_created > (30 - days_threshold)
    }
    
    /// Get a value from the token metadata
    pub fn get_metadata<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T> {
        match self.metadata.get(key) {
            Some(value) => serde_json::from_value(value.clone())
                .map_err(|e| anyhow!("Failed to deserialize metadata '{}': {}", key, e)),
            None => Err(anyhow!("Metadata key '{}' not found", key)),
        }
    }
    
    /// Set a value in the token metadata
    pub fn set_metadata<T: Serialize>(&mut self, key: &str, value: T) -> Result<()> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| anyhow!("Failed to serialize metadata '{}': {}", key, e))?;
        self.metadata.insert(key.to_string(), json_value);
        Ok(())
    }
    
    /// Calculate seconds until expiration
    pub fn seconds_until_expiration(&self) -> Option<i64> {
        self.expiration.map(|exp| (exp - Utc::now()).num_seconds())
    }
}

/// Enum representing different authentication states for a provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthState {
    /// The provider is not authenticated
    Unauthenticated,
    
    /// Authentication is in progress
    Authenticating { 
        flow_id: String 
    },
    
    /// Successfully authenticated with a valid token
    Authenticated { 
        token: TokenData 
    },
    
    /// Token is being refreshed
    Refreshing { 
        previous_token: TokenData 
    },
    
    /// Authentication failed
    Failed { 
        reason: String, 
        recoverable: bool 
    },
}

/// Enum representing different authentication events for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthEvent {
    /// Authentication state changed
    StateChanged { 
        provider: String, 
        state: AuthState 
    },
    
    /// Token refresh has been scheduled
    RefreshScheduled { 
        provider: String, 
        when: DateTime<Utc> 
    },
    
    /// Token is expiring soon and needs refresh
    TokenExpiringWarning { 
        provider: String, 
        expires_in_secs: u64 
    },
    
    /// Recovery was attempted
    RecoveryAttempted { 
        provider: String, 
        success: bool 
    },
    
    /// User action is required to resolve an auth issue
    UserActionRequired { 
        provider: String, 
        action: String 
    },
}