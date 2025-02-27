use anyhow::{anyhow, Result};
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
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Additional metadata about the token
    pub metadata: HashMap<String, Value>,
}

impl TokenData {
    /// Create a new token data instance
    pub fn new(access_token: String, refresh_token: Option<String>) -> Self {
        Self {
            access_token,
            refresh_token,
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires) => expires <= chrono::Utc::now(),
            None => false, // If we don't know when it expires, assume it's still valid
        }
    }

    /// Set the token expiration time
    pub fn set_expiration(&mut self, expires_in_secs: u64) {
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in_secs as i64);
        self.expires_at = Some(expires_at);
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
                }
            }
        }
        false
    }

    /// Check if the token will expire soon (within the given seconds)
    pub fn expires_soon(&self, within_seconds: u64) -> bool {
        match self.expires_at {
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
        let mut app_guard = self.app.write().await;
        *app_guard = Some(app);
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

        // Convert tokens to JSON
        let token_json = json!({
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "expires_at": tokens.expires_at,
            "metadata": tokens.metadata
        });

        // Store the tokens securely
        store.set(&secure_key, token_json);

        // Save the store
        if let Err(e) = store.save() {
            error!(error = %e, "Failed to save secure store");
            return Err(anyhow!("Failed to save secure store: {}", e));
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

                let expires_at = value.get("expires_at").and_then(|v| {
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
                            let map: HashMap<String, Value> =
                                serde_json::from_value(v.clone()).unwrap_or_default();
                            Some(map)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                // Create TokenData
                let token_data = TokenData {
                    access_token,
                    refresh_token,
                    expires_at,
                    metadata,
                };

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

        // Save the store to persist the deletion
        if let Err(e) = store.save() {
            error!(error = %e, "Failed to save secure store after deletion");
            return Err(anyhow!("Failed to save secure store: {}", e));
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
