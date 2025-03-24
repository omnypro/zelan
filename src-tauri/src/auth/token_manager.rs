use anyhow::{anyhow, Result};
use fastrand;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Secure storage filename
const SECURE_STORE_FILENAME: &str = "zelan.secure.json";

/// Token data storage structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    /// Access token for the service
    pub access_token: String,
    /// Optional refresh token
    pub refresh_token: Option<String>,
    /// When the token expires (if known)
    pub expires_in: Option<chrono::DateTime<chrono::Utc>>,
    /// Additional metadata about the token
    pub metadata: HashMap<String, Value>,
}

impl TokenData {
    /// Create a new token data instance
    pub fn new(access_token: String, refresh_token: Option<String>) -> Self {
        Self {
            access_token,
            refresh_token,
            expires_in: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        match self.expires_in {
            Some(expires) => expires <= chrono::Utc::now(),
            None => false, // If we don't know when it expires, assume it's still valid
        }
    }

    /// Set the token expiration time
    pub fn set_expiration(&mut self, expires_in_secs: u64) {
        let expires_in = chrono::Utc::now() + chrono::Duration::seconds(expires_in_secs as i64);
        self.expires_in = Some(expires_in);
        
        // Also store the expiration time in metadata for better tracking
        self.set_metadata_value(
            "expires_at",
            serde_json::Value::String(expires_in.to_rfc3339()),
        );
        
        // Store the original seconds value for diagnostics
        self.set_metadata_value(
            "expires_in_seconds",
            serde_json::Value::Number(serde_json::Number::from(expires_in_secs)),
        );
    }

    /// Set the token metadata value
    pub fn set_metadata_value(&mut self, key: &str, value: Value) {
        self.metadata.insert(key.to_string(), value);
    }

    /// Get a metadata value
    pub fn get_metadata_value(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    /// Track refresh token creation time to monitor 30-day expiry
    pub fn track_refresh_token_created(&mut self) {
        let now = chrono::Utc::now();
        self.set_metadata_value(
            "refresh_token_created_at",
            serde_json::Value::String(now.to_rfc3339()),
        );
    }

    /// Check if refresh token is approaching 30-day expiry limit
    pub fn refresh_token_expires_soon(&self, within_days: u64) -> bool {
        if let Some(created_str) = self.get_metadata_value("refresh_token_created_at") {
            if let Some(created_str) = created_str.as_str() {
                if let Ok(created) = chrono::DateTime::parse_from_rfc3339(created_str) {
                    let created_utc = created.with_timezone(&chrono::Utc);
                    let now = chrono::Utc::now();
                    let age = now - created_utc;

                    // Check if token is within the specified days of reaching 30-day expiry
                    let days_until_expiry = 30 - age.num_days();
                    return days_until_expiry < within_days as i64;
                } else {
                    // Log parsing error for better diagnostics
                    warn!("Failed to parse refresh token creation date: {}", created_str);
                }
            }
        }
        false
    }

    /// Check if the token will expire soon (within the given seconds)
    pub fn expires_soon(&self, within_seconds: u64) -> bool {
        match self.expires_in {
            Some(expires) => {
                let now = chrono::Utc::now();
                let expires_in = expires - now;
                expires_in.num_seconds() < within_seconds as i64
            }
            None => false,
        }
    }
}

/// Token Manager for handling authentication tokens securely
pub struct TokenManager {
    /// Application handle for accessing secure storage
    app: Arc<RwLock<Option<AppHandle>>>,
    /// In-memory token cache
    tokens: Arc<RwLock<HashMap<String, TokenData>>>,
}

impl Clone for TokenManager {
    fn clone(&self) -> Self {
        // IMPORTANT: Properly share the same instances of RwLocks
        // This ensures all clones have consistent state and can see
        // updates made by any instance
        Self {
            app: Arc::clone(&self.app),
            tokens: Arc::clone(&self.tokens),
        }
    }
}

impl TokenManager {
    /// Create a new token manager instance
    pub fn new() -> Self {
        Self {
            app: Arc::new(RwLock::new(None)),
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize with an app handle for secure storage access
    pub fn with_app(app: AppHandle) -> Self {
        Self {
            app: Arc::new(RwLock::new(Some(app))),
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set the app handle after initialization
    /// Now this can be used with an immutable reference
    pub async fn set_app(&self, app: AppHandle) {
        info!("Setting app handle on TokenManager");
        let mut app_guard = self.app.write().await;
        *app_guard = Some(app);
        info!("App handle successfully set on TokenManager");
    }

    /// Store tokens for a specific adapter
    pub async fn store_tokens(&self, adapter_name: &str, tokens: TokenData) -> Result<()> {
        // Update in-memory cache
        {
            let mut token_map = self.tokens.write().await;
            token_map.insert(adapter_name.to_string(), tokens.clone());
        }

        // Update secure storage if app is available
        let app_guard = self.app.read().await;
        if let Some(app) = &*app_guard {
            self.store_tokens_secure(app, adapter_name, &tokens).await?;
        } else {
            warn!(
                "App handle not set, tokens only stored in memory for adapter: {}",
                adapter_name
            );
        }

        Ok(())
    }

    /// Store tokens in secure storage
    async fn store_tokens_secure(
        &self,
        app: &AppHandle,
        adapter_name: &str,
        tokens: &TokenData,
    ) -> Result<()> {
        debug!(adapter = %adapter_name, "Storing tokens securely");

        // Create a secure store key specific to this adapter
        let secure_key = format!("secure_{}_tokens", adapter_name);

        // Get the store
        let store = app.store(SECURE_STORE_FILENAME).map_err(|e| {
            error!(error = %e, adapter = %adapter_name, "Failed to access secure store");
            anyhow!("Failed to access secure store: {}", e)
        })?;

        // Convert tokens to JSON, ensuring expires_in is properly serialized as a string
        let expires_in_str = tokens.expires_in.map(|dt| dt.to_rfc3339());
        let token_json = json!({
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "expires_in": expires_in_str,
            "metadata": tokens.metadata
        });

        // Store the tokens securely
        store.set(&secure_key, token_json);

        // Save the store with retry
        let mut attempts = 0;
        const MAX_ATTEMPTS: u8 = 3;
        let mut last_error = None;

        while attempts < MAX_ATTEMPTS {
            match store.save() {
                Ok(_) => {
                    if attempts > 0 {
                        info!("Successfully saved secure store after {} retries", attempts);
                    }
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    warn!(error = %e, attempt = attempts, "Secure store save failed");
                    last_error = Some(e);
                    
                    if attempts < MAX_ATTEMPTS {
                        // Exponential backoff with jitter
                        let base_delay = 50 * 2u64.pow(attempts as u32);
                        let jitter = fastrand::u64(0..50);
                        let delay = base_delay + jitter;
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        if let Some(e) = last_error {
            if attempts >= MAX_ATTEMPTS {
                error!(error = %e, "Failed to save secure store after maximum attempts");
                return Err(anyhow!("Failed to save secure store after {} attempts: {}", MAX_ATTEMPTS, e));
            }
        }

        info!(adapter = %adapter_name, "Tokens securely stored");
        Ok(())
    }

    /// Retrieve tokens for a specific adapter
    pub async fn get_tokens(&self, adapter_name: &str) -> Result<Option<TokenData>> {
        // Check in-memory cache first
        {
            let token_map = self.tokens.read().await;
            if let Some(tokens) = token_map.get(adapter_name) {
                return Ok(Some(tokens.clone()));
            }
        }

        // Try to load from secure storage if app is available
        let app_guard = self.app.read().await;
        if let Some(app) = &*app_guard {
            if let Some(tokens) = self.retrieve_tokens_secure(app, adapter_name).await? {
                // Update cache with retrieved tokens
                let mut token_map = self.tokens.write().await;
                token_map.insert(adapter_name.to_string(), tokens.clone());
                return Ok(Some(tokens));
            }
        }

        // Not found in memory or storage
        Ok(None)
    }

    /// Retrieve tokens from secure storage
    async fn retrieve_tokens_secure(
        &self,
        app: &AppHandle,
        adapter_name: &str,
    ) -> Result<Option<TokenData>> {
        debug!(adapter = %adapter_name, "Retrieving tokens from secure storage");

        // Create a secure store key specific to this adapter
        let secure_key = format!("secure_{}_tokens", adapter_name);

        // Get the store
        let store = app.store(SECURE_STORE_FILENAME).map_err(|e| {
            error!(error = %e, adapter = %adapter_name, "Failed to access secure store");
            anyhow!("Failed to access secure store: {}", e)
        })?;

        // Check if we have tokens for this adapter
        if !store.has(&secure_key) {
            debug!(adapter = %adapter_name, "No secure tokens found");
            return Ok(None);
        }

        // Get the tokens
        let token_value: Option<Value> = store.get(&secure_key).map(|v| v.clone());

        match token_value {
            Some(value) => {
                debug!(adapter = %adapter_name, "Found secure tokens");

                // Extract token fields
                let access_token = value
                    .get("access_token")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Invalid token data: missing access_token"))?
                    .to_string();

                let refresh_token = value.get("refresh_token").and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        v.as_str().map(|s| s.to_string())
                    }
                });

                let expires_in = value.get("expires_in").and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        v.as_str()
                            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    }
                });

                let metadata = value
                    .get("metadata")
                    .and_then(|v| {
                        if v.is_object() {
                            match serde_json::from_value(v.clone()) {
                                Ok(map) => Some(map),
                                Err(e) => {
                                    error!(error = %e, "Failed to parse token metadata");
                                    None
                                }
                            }
                        } else {
                            warn!("Token metadata is not an object");
                            None
                        }
                    })
                    .unwrap_or_default();

                // Create TokenData
                let token_data = TokenData {
                    access_token,
                    refresh_token,
                    expires_in,
                    metadata,
                };

                // Validate the restored token
                if token_data.access_token.is_empty() {
                    warn!(adapter = %adapter_name, "Restored token has empty access_token");
                    return Ok(None);
                }

                // Check if the token is already expired
                if token_data.is_expired() {
                    info!(adapter = %adapter_name, "Restored token is already expired");
                    // We still return it, as it might have a valid refresh token
                }

                Ok(Some(token_data))
            }
            None => {
                debug!(adapter = %adapter_name, "Token key exists but no value found");
                Ok(None)
            }
        }
    }

    /// Remove tokens for a specific adapter
    pub async fn remove_tokens(&self, adapter_name: &str) -> Result<()> {
        // Remove from in-memory cache
        {
            let mut token_map = self.tokens.write().await;
            token_map.remove(adapter_name);
        }

        // Remove from secure storage if app is available
        let app_guard = self.app.read().await;
        if let Some(app) = &*app_guard {
            self.remove_tokens_secure(app, adapter_name).await?;
        }

        Ok(())
    }

    /// Remove tokens from secure storage
    async fn remove_tokens_secure(&self, app: &AppHandle, adapter_name: &str) -> Result<()> {
        debug!(adapter = %adapter_name, "Removing tokens from secure storage");

        // Create a secure store key specific to this adapter
        let secure_key = format!("secure_{}_tokens", adapter_name);

        // Get the store
        let store = app.store(SECURE_STORE_FILENAME).map_err(|e| {
            error!(error = %e, adapter = %adapter_name, "Failed to access secure store");
            anyhow!("Failed to access secure store: {}", e)
        })?;

        // Check if tokens exist
        if !store.has(&secure_key) {
            debug!(adapter = %adapter_name, "No secure tokens to remove");
            return Ok(());
        }

        // Remove the tokens
        let deleted = store.delete(&secure_key);
        debug!(deleted = %deleted, "Deletion result");

        // Save the store with retry
        let mut attempts = 0;
        const MAX_ATTEMPTS: u8 = 3;
        let mut last_error = None;

        while attempts < MAX_ATTEMPTS {
            match store.save() {
                Ok(_) => {
                    if attempts > 0 {
                        info!("Successfully saved secure store after {} retries", attempts);
                    }
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    warn!(error = %e, attempt = attempts, "Secure store save failed after deletion");
                    last_error = Some(e);
                    
                    if attempts < MAX_ATTEMPTS {
                        // Exponential backoff with jitter
                        let base_delay = 50 * 2u64.pow(attempts as u32);
                        let jitter = fastrand::u64(0..50);
                        let delay = base_delay + jitter;
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        if let Some(e) = last_error {
            if attempts >= MAX_ATTEMPTS {
                error!(error = %e, "Failed to save secure store after deletion and maximum attempts");
                return Err(anyhow!("Failed to save secure store after deletion: {}", e));
            }
        }

        info!(adapter = %adapter_name, "Secure tokens removed");
        Ok(())
    }

    /// Update tokens for a specific adapter
    pub async fn update_tokens(
        &self,
        adapter_name: &str,
        update_fn: impl FnOnce(TokenData) -> TokenData,
    ) -> Result<()> {
        // Get current tokens
        let current_tokens = match self.get_tokens(adapter_name).await? {
            Some(tokens) => tokens,
            None => return Err(anyhow!("No tokens found for adapter: {}", adapter_name)),
        };

        // Apply update function
        let updated_tokens = update_fn(current_tokens);

        // Store updated tokens
        self.store_tokens(adapter_name, updated_tokens).await?;

        Ok(())
    }

    /// Check if tokens exist and are valid for an adapter
    pub async fn has_valid_tokens(&self, adapter_name: &str) -> bool {
        match self.get_tokens(adapter_name).await {
            Ok(Some(tokens)) => !tokens.is_expired(),
            _ => false,
        }
    }

    /// Check if tokens will expire soon for an adapter
    pub async fn tokens_expire_soon(&self, adapter_name: &str, within_seconds: u64) -> bool {
        match self.get_tokens(adapter_name).await {
            Ok(Some(tokens)) => tokens.expires_soon(within_seconds),
            _ => false,
        }
    }

    /// Check if refresh tokens are approaching their 30-day expiry for an adapter
    pub async fn refresh_tokens_expire_soon(&self, adapter_name: &str, within_days: u64) -> bool {
        match self.get_tokens(adapter_name).await {
            Ok(Some(tokens)) => tokens.refresh_token_expires_soon(within_days),
            _ => false,
        }
    }
    
    /// Ensures tokens have an expiration time set
    /// If no expiration time is set, uses a default token lifetime
    pub async fn ensure_token_expiration(&self, adapter_name: &str, default_expiry_secs: u64) -> Result<()> {
        info!(adapter = %adapter_name, "Ensuring token has expiration time set");
        
        // Get the current tokens
        match self.get_tokens(adapter_name).await? {
            Some(mut token_data) => {
                // Check if token has an expiration time
                if token_data.expires_in.is_none() {
                    info!(
                        adapter = %adapter_name, 
                        "Token has no expiration time, setting default of {} seconds", 
                        default_expiry_secs
                    );
                    
                    // Set default expiration time
                    token_data.set_expiration(default_expiry_secs);
                    
                    // Store the updated token
                    self.store_tokens(adapter_name, token_data).await?;
                    
                    info!(adapter = %adapter_name, "Successfully updated token with default expiration time");
                } else {
                    debug!(adapter = %adapter_name, "Token already has expiration time set");
                }
                
                Ok(())
            },
            None => {
                debug!(adapter = %adapter_name, "No tokens found to update");
                Ok(())
            }
        }
    }
    
    /// Specifically ensure Twitch tokens have an expiration time
    /// The default Twitch token lifetime is 4 hours (14400 seconds)
    pub async fn ensure_twitch_token_expiration(&self) -> Result<()> {
        // Default Twitch token lifetime is 4 hours (14400 seconds)
        const DEFAULT_TWITCH_TOKEN_EXPIRY: u64 = 14400;
        self.ensure_token_expiration("twitch", DEFAULT_TWITCH_TOKEN_EXPIRY).await
    }

    /// Attempt to recover tokens with validation and error handling
    /// This provides a simple one-step method for secure token recovery
    pub async fn recover_tokens_with_validation(&self, adapter_name: &str) -> Result<Option<TokenData>> {
        info!(adapter = %adapter_name, "Attempting to recover tokens with validation");
        
        // Get app handle
        let app_guard = self.app.read().await;
        let app = match &*app_guard {
            Some(app) => app,
            None => {
                warn!(adapter = %adapter_name, "No app handle available for token recovery");
                return Ok(None);
            }
        };
        
        // Attempt to retrieve tokens with retries
        let mut attempts = 0;
        const MAX_ATTEMPTS: u8 = 3;
        let mut last_error = None;
        
        while attempts < MAX_ATTEMPTS {
            match self.retrieve_tokens_secure(app, adapter_name).await {
                Ok(Some(tokens)) => {
                    // Successfully retrieved tokens
                    
                    // Validate access token
                    if tokens.access_token.is_empty() {
                        warn!(adapter = %adapter_name, "Recovered tokens have empty access_token");
                        return Ok(None);
                    }
                    
                    // Update in-memory cache
                    let mut token_map = self.tokens.write().await;
                    token_map.insert(adapter_name.to_string(), tokens.clone());
                    
                    info!(adapter = %adapter_name, "Successfully recovered and validated tokens");
                    return Ok(Some(tokens));
                }
                Ok(None) => {
                    // No tokens found
                    info!(adapter = %adapter_name, "No tokens found to recover");
                    return Ok(None);
                }
                Err(e) => {
                    attempts += 1;
                    warn!(error = %e, attempt = attempts, "Token recovery failed");
                    last_error = Some(e);
                    
                    if attempts < MAX_ATTEMPTS {
                        // Exponential backoff with jitter
                        let base_delay = 100 * 2u64.pow(attempts as u32);
                        let jitter = fastrand::u64(0..100);
                        let delay = base_delay + jitter;
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }
        
        // All attempts failed
        if let Some(e) = last_error {
            error!(error = %e, adapter = %adapter_name, "Failed to recover tokens after maximum attempts");
            Err(anyhow!("Token recovery failed after {} attempts: {}", MAX_ATTEMPTS, e))
        } else {
            // This shouldn't happen with the logic above, but just in case
            Ok(None)
        }
    }
}

impl Default for TokenManager {
    fn default() -> Self {
        Self::new()
    }
}

// Allow cloning the TokenManager
// This is safe because all state is stored in Arc<RwLock<>>
impl Clone for TokenManager {
    fn clone(&self) -> Self {
        Self {
            app: Arc::clone(&self.app),
            tokens: Arc::clone(&self.tokens),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_token_expiration() {
        let mut token = TokenData::new("test_token".to_string(), Some("refresh_token".to_string()));
        
        // Test with non-expired token
        token.set_expiration(3600); // 1 hour from now
        assert!(!token.is_expired());
        assert!(!token.expires_soon(10)); // Not expiring within 10 seconds
        assert!(token.expires_soon(4000)); // Expiring within 4000 seconds
        
        // Test with expired token
        let expired_time = Utc::now() - chrono::Duration::seconds(100);
        token.expires_in = Some(expired_time);
        assert!(token.is_expired());
        assert!(token.expires_soon(10)); // Already expired
    }
    
    #[test]
    fn test_refresh_token_expiry() {
        let mut token = TokenData::new("test_token".to_string(), Some("refresh_token".to_string()));
        
        // Set refresh token created time to 25 days ago
        let created_time = Utc::now() - chrono::Duration::days(25);
        token.set_metadata_value(
            "refresh_token_created_at",
            serde_json::Value::String(created_time.to_rfc3339()),
        );
        
        // Should expire soon (within 7 days)
        assert!(token.refresh_token_expires_soon(7));
        
        // Should not expire soon (within 3 days)
        assert!(!token.refresh_token_expires_soon(3));
        
        // Set to 28 days ago (very close to 30-day limit)
        let created_time = Utc::now() - chrono::Duration::days(28);
        token.set_metadata_value(
            "refresh_token_created_at",
            serde_json::Value::String(created_time.to_rfc3339()),
        );
        
        // Should expire very soon
        assert!(token.refresh_token_expires_soon(3));
    }
    
    #[test]
    fn test_invalid_refresh_token_date() {
        let mut token = TokenData::new("test_token".to_string(), Some("refresh_token".to_string()));
        
        // Set invalid date format
        token.set_metadata_value(
            "refresh_token_created_at",
            serde_json::Value::String("not-a-valid-date".to_string()),
        );
        
        // Should handle invalid date without panicking
        assert!(!token.refresh_token_expires_soon(7));
        
        // Set non-string value
        token.set_metadata_value(
            "refresh_token_created_at",
            serde_json::Value::Number(serde_json::Number::from(12345)),
        );
        
        // Should handle non-string value without panicking
        assert!(!token.refresh_token_expires_soon(7));
    }
    
    // Integration tests would need a mock app handle for secure storage
    #[tokio::test]
    async fn test_token_manager_basic() {
        let token_manager = TokenManager::new();
        let adapter_name = "test_adapter";
        let token_data = TokenData::new("test_access".to_string(), Some("test_refresh".to_string()));
        
        // Test in-memory storage without app handle
        assert!(token_manager.store_tokens(adapter_name, token_data.clone()).await.is_ok());
        
        // Retrieve from memory
        let retrieved = token_manager.get_tokens(adapter_name).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.access_token, "test_access");
        assert_eq!(retrieved.refresh_token, Some("test_refresh".to_string()));
        
        // Test token validation
        assert!(token_manager.has_valid_tokens(adapter_name).await);
        
        // Make the token "expire"
        token_manager.update_tokens(adapter_name, |mut t| {
            t.expires_in = Some(Utc::now() - chrono::Duration::seconds(10));
            t
        }).await.unwrap();
        
        // Should now be invalid
        assert!(!token_manager.has_valid_tokens(adapter_name).await);
        
        // Test removal
        token_manager.remove_tokens(adapter_name).await.unwrap();
        assert!(!token_manager.has_valid_tokens(adapter_name).await);
    }
}
