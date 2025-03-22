use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Stores token data for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    /// The access token for API requests
    pub access_token: String,
    
    /// The refresh token for refreshing the access token (optional for app tokens)
    pub refresh_token: Option<String>,
    
    /// When the token expires (in seconds since UNIX epoch)
    pub expires_at: u64,
    
    /// Additional metadata for the token
    pub metadata: serde_json::Value,
}

impl TokenData {
    /// Create a new token data instance
    pub fn new(
        access_token: String,
        refresh_token: Option<String>,
        expires_in: u64,
        metadata: serde_json::Value,
    ) -> Self {
        // Calculate expiration time from duration
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        
        let expires_at = now + expires_in;
        
        TokenData {
            access_token,
            refresh_token,
            expires_at,
            metadata,
        }
    }
    
    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        
        self.expires_at <= now
    }
    
    /// Get the time remaining before the token expires
    pub fn time_remaining(&self) -> Duration {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        
        if self.expires_at <= now {
            Duration::from_secs(0)
        } else {
            Duration::from_secs(self.expires_at - now)
        }
    }
    
    /// Check if the token needs to be refreshed (less than 10 minutes remaining)
    pub fn needs_refresh(&self) -> bool {
        self.time_remaining() < Duration::from_secs(10 * 60)
    }
}

/// Manages authentication tokens securely
pub struct TokenManager {
    data_dir: PathBuf,
}

impl TokenManager {
    /// Create a new token manager with the given data directory
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        let data_dir = data_dir.as_ref().to_path_buf();
        
        // Ensure the directory exists
        fs::create_dir_all(&data_dir).unwrap_or_else(|e| {
            eprintln!("Failed to create token data directory: {}", e);
        });
        
        TokenManager { data_dir }
    }
    
    /// Get the path for a token file
    fn get_token_path(&self, service: &str, identifier: &str) -> PathBuf {
        self.data_dir.join(format!("{}-{}.token", service, identifier))
    }
    
    /// Store a token for a service
    pub fn store_token(&self, service: &str, identifier: &str, token: &TokenData) -> Result<()> {
        let token_path = self.get_token_path(service, identifier);
        
        // Serialize the token data
        let token_json = serde_json::to_string(token)
            .map_err(|e| Error::serialization(format!("Failed to serialize token: {}", e)))?;
        
        // Write to file
        fs::write(&token_path, token_json)
            .map_err(|e| Error::io(format!("Failed to write token file: {}", e)))?;
        
        Ok(())
    }
    
    /// Retrieve a token for a service
    pub fn get_token(&self, service: &str, identifier: &str) -> Result<TokenData> {
        let token_path = self.get_token_path(service, identifier);
        
        // Check if the file exists
        if !token_path.exists() {
            return Err(Error::not_found(format!(
                "Token for {}/{} not found",
                service, identifier
            )));
        }
        
        // Read the file
        let token_json = fs::read_to_string(&token_path)
            .map_err(|e| Error::io(format!("Failed to read token file: {}", e)))?;
        
        // Deserialize the token data
        let token: TokenData = serde_json::from_str(&token_json)
            .map_err(|e| Error::serialization(format!("Failed to deserialize token: {}", e)))?;
        
        Ok(token)
    }
    
    /// Remove a token for a service
    pub fn remove_token(&self, service: &str, identifier: &str) -> Result<()> {
        let token_path = self.get_token_path(service, identifier);
        
        // Check if the file exists
        if !token_path.exists() {
            return Ok(());
        }
        
        // Remove the file
        fs::remove_file(&token_path)
            .map_err(|e| Error::io(format!("Failed to remove token file: {}", e)))?;
        
        Ok(())
    }
}