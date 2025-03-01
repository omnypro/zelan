use std::sync::Arc;
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};
use tauri_plugin_store::Store;
use tracing::{debug, error, info};

use crate::auth::token::TokenData;

/// Structure for storing tokens with integrity checks
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenStoragePayload {
    /// The token data
    token: TokenData,
    /// When this storage entry was created
    created_at: chrono::DateTime<Utc>,
    /// Hash to verify token integrity
    integrity_hash: String,
}

/// Secure storage provider for authentication tokens
pub struct SecureTokenStore {
    app_handle: Option<Arc<AppHandle>>,
    store_name: String,
}

impl SecureTokenStore {
    /// Create a new secure token store
    pub fn new(store_name: &str) -> Self {
        Self {
            app_handle: None,
            store_name: store_name.to_string(),
        }
    }
    
    /// Set the app handle for accessing Tauri secure storage
    pub fn with_app_handle(mut self, app_handle: Arc<AppHandle>) -> Self {
        self.app_handle = Some(app_handle);
        self
    }
    
    /// Store a token securely
    pub async fn store(&self, provider: &str, token: &TokenData) -> Result<()> {
        let app = self.get_app()?;
        let store = app.state::<Arc<Store<tauri::AppHandle>>>();
        
        // Create secure key specific to provider
        let secure_key = self.make_storage_key(provider);
        
        // Store with integrity hash
        let storage_payload = TokenStoragePayload {
            token: token.clone(),
            created_at: Utc::now(),
            integrity_hash: self.calculate_integrity_hash(token)?,
        };
        
        // Store in Tauri's secure store
        store.set(&secure_key, serde_json::to_value(storage_payload)?)?;
        store.save()?;
        
        info!(provider = provider, "Token stored securely");
        Ok(())
    }
    
    /// Retrieve a token from secure storage
    pub async fn retrieve(&self, provider: &str) -> Result<Option<TokenData>> {
        let app = self.get_app()?;
        let store = app.state::<Arc<Store<tauri::AppHandle>>>();
        
        // Get the storage key
        let secure_key = self.make_storage_key(provider);
        
        // Check if we have a stored token
        if !store.has(&secure_key)? {
            debug!(provider = provider, "No token found in secure storage");
            return Ok(None);
        }
        
        // Retrieve the token payload
        let payload = match store.get(&secure_key)? {
            Some(value) => serde_json::from_value::<TokenStoragePayload>(value)
                .map_err(|e| anyhow!("Failed to deserialize token payload: {}", e))?,
            None => return Ok(None),
        };
        
        // Verify integrity
        let expected_hash = self.calculate_integrity_hash(&payload.token)?;
        if expected_hash != payload.integrity_hash {
            error!(
                provider = provider,
                "Token integrity check failed, possible tampering"
            );
            return Err(anyhow!("Token integrity check failed"));
        }
        
        debug!(provider = provider, "Token retrieved from secure storage");
        Ok(Some(payload.token))
    }
    
    /// Clear a token from secure storage
    pub async fn clear(&self, provider: &str) -> Result<()> {
        let app = self.get_app()?;
        let store = app.state::<Arc<Store<tauri::AppHandle>>>();
        
        // Get the storage key
        let secure_key = self.make_storage_key(provider);
        
        // Remove the token if it exists
        if store.has(&secure_key)? {
            store.delete(&secure_key)?;
            store.save()?;
            info!(provider = provider, "Token removed from secure storage");
        } else {
            debug!(provider = provider, "No token to remove from secure storage");
        }
        
        Ok(())
    }
    
    /// Clear all tokens from secure storage
    pub async fn clear_all(&self) -> Result<()> {
        let app = self.get_app()?;
        let store = app.state::<Arc<Store<tauri::AppHandle>>>();
        
        // Get all keys and filter for auth tokens
        let keys: Vec<String> = store
            .keys()?
            .into_iter()
            .filter(|k| k.starts_with("auth_token_"))
            .collect();
        
        // Delete each token
        for key in keys {
            store.delete(&key)?;
        }
        
        // Save changes
        store.save()?;
        info!("All tokens cleared from secure storage");
        
        Ok(())
    }
    
    /// Generate the storage key for a provider
    fn make_storage_key(&self, provider: &str) -> String {
        format!("auth_token_{}", provider)
    }
    
    /// Calculate an integrity hash for the token
    fn calculate_integrity_hash(&self, token: &TokenData) -> Result<String> {
        // Create a string with all critical token fields
        let token_string = format!(
            "{}:{}:{}:{}:{}",
            token.access_token,
            token.refresh_token.as_deref().unwrap_or(""),
            token.provider,
            token.user_id.as_deref().unwrap_or(""),
            token.expiration.map_or_else(String::new, |e| e.to_rfc3339())
        );
        
        // Hash the token data
        let mut hasher = Sha256::new();
        hasher.update(token_string.as_bytes());
        let hash = hasher.finalize();
        
        // Return hex string
        Ok(format!("{:x}", hash))
    }
    
    /// Get the app handle or return an error
    fn get_app(&self) -> Result<Arc<AppHandle>> {
        match &self.app_handle {
            Some(app) => Ok(app.clone()),
            None => Err(anyhow!("App handle not set for SecureTokenStore")),
        }
    }
}