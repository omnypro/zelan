use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info};

// Default configuration values
const DEFAULT_WS_PORT: u16 = 9000;
const DEFAULT_API_PORT: u16 = 9001;
const DEFAULT_MAX_CONNECTIONS: usize = 100;
const DEFAULT_TIMEOUT_SECONDS: u64 = 300;
const DEFAULT_PING_INTERVAL_SECONDS: u64 = 60;

/// Main configuration struct for Zelan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// WebSocket server configuration
    pub websocket: WebSocketConfig,
    /// API server configuration
    pub api: ApiConfig,
    /// Adapter settings keyed by adapter name
    pub adapters: HashMap<String, AdapterSettings>,
    /// Authentication configuration
    pub auth: AuthConfig,
}

/// WebSocket server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Port to bind the WebSocket server to
    #[serde(default = "default_ws_port")]
    pub port: u16,
    /// Maximum number of simultaneous connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    /// Inactivity timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Ping interval in seconds
    #[serde(default = "default_ping_interval")]
    pub ping_interval: u64,
}

/// API server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Port to bind the API server to
    #[serde(default = "default_api_port")]
    pub port: u16,
    /// Whether to enable CORS
    #[serde(default = "default_cors_enabled")]
    pub cors_enabled: bool,
    /// Allowed origins for CORS (empty means all)
    #[serde(default)]
    pub cors_origins: Vec<String>,
    /// Whether to enable the API documentation
    #[serde(default = "default_swagger_enabled")]
    pub swagger_enabled: bool,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Whether authentication is required for API access
    #[serde(default = "default_auth_required")]
    pub auth_required: bool,
    /// JWT signing key (default is randomly generated)
    #[serde(default = "generate_random_key")]
    pub jwt_key: String,
    /// JWT token expiration in seconds
    #[serde(default = "default_jwt_expiration")]
    pub jwt_expiration_seconds: u64,
    /// Whether to allow API keys
    #[serde(default = "default_api_keys_enabled")]
    pub api_keys_enabled: bool,
    /// Maximum number of API keys per client
    #[serde(default = "default_max_api_keys")]
    pub max_api_keys: usize,
}

/// Settings for a service adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterSettings {
    /// Whether the adapter is enabled
    pub enabled: bool,
    /// Adapter-specific configuration
    #[serde(default)]
    pub config: serde_json::Value,
    /// Display name for the adapter
    pub display_name: String,
    /// Description of the adapter's functionality
    pub description: String,
}

// Default functions
fn default_ws_port() -> u16 {
    std::env::var("ZELAN_WS_PORT")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(DEFAULT_WS_PORT)
}

fn default_api_port() -> u16 {
    std::env::var("ZELAN_API_PORT")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(DEFAULT_API_PORT)
}

fn default_max_connections() -> usize {
    std::env::var("ZELAN_MAX_CONNECTIONS")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(DEFAULT_MAX_CONNECTIONS)
}

fn default_timeout() -> u64 {
    std::env::var("ZELAN_TIMEOUT_SECONDS")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECONDS)
}

fn default_ping_interval() -> u64 {
    std::env::var("ZELAN_PING_INTERVAL")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(DEFAULT_PING_INTERVAL_SECONDS)
}

fn default_cors_enabled() -> bool {
    std::env::var("ZELAN_CORS_ENABLED")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(true)
}

fn default_swagger_enabled() -> bool {
    std::env::var("ZELAN_SWAGGER_ENABLED")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(true)
}

fn default_auth_required() -> bool {
    std::env::var("ZELAN_AUTH_REQUIRED")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(false)
}

fn generate_random_key() -> String {
    use rand::Rng;
    let key: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    
    std::env::var("ZELAN_JWT_KEY").unwrap_or(key)
}

fn default_jwt_expiration() -> u64 {
    std::env::var("ZELAN_JWT_EXPIRATION")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(86400) // 24 hours
}

fn default_api_keys_enabled() -> bool {
    std::env::var("ZELAN_API_KEYS_ENABLED")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(true)
}

fn default_max_api_keys() -> usize {
    std::env::var("ZELAN_MAX_API_KEYS")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(5)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            websocket: WebSocketConfig::default(),
            api: ApiConfig::default(),
            adapters: HashMap::new(),
            auth: AuthConfig::default(),
        }
    }
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            port: default_ws_port(),
            max_connections: default_max_connections(),
            timeout_seconds: default_timeout(),
            ping_interval: default_ping_interval(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            port: default_api_port(),
            cors_enabled: default_cors_enabled(),
            cors_origins: Vec::new(),
            swagger_enabled: default_swagger_enabled(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            auth_required: default_auth_required(),
            jwt_key: generate_random_key(),
            jwt_expiration_seconds: default_jwt_expiration(),
            api_keys_enabled: default_api_keys_enabled(),
            max_api_keys: default_max_api_keys(),
        }
    }
}

/// Manages configuration for the application
#[derive(Debug, Clone)]
pub struct ConfigManager {
    config: Arc<RwLock<Config>>,
    config_path: PathBuf,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub async fn new() -> Result<Self> {
        // Determine configuration path
        let config_path = get_config_path()?;
        let config = load_or_create_config(&config_path).await?;
        
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
        })
    }
    
    /// Get a clone of the current configuration
    pub async fn get_config(&self) -> Config {
        self.config.read().await.clone()
    }
    
    /// Update the configuration
    pub async fn update_config(&self, new_config: Config) -> Result<()> {
        // Update in memory
        *self.config.write().await = new_config.clone();
        
        // Save to disk
        save_config(&self.config_path, &new_config).await?;
        
        Ok(())
    }
    
    /// Update a specific adapter's settings
    pub async fn update_adapter_settings(&self, name: &str, settings: AdapterSettings) -> Result<()> {
        let mut config = self.config.write().await;
        config.adapters.insert(name.to_string(), settings);
        
        // Save to disk
        save_config(&self.config_path, &config).await?;
        
        Ok(())
    }
}

/// Load the application configuration
pub async fn load_config() -> Result<Config> {
    let config_manager = ConfigManager::new().await?;
    Ok(config_manager.get_config().await)
}

/// Get the path to the configuration file
fn get_config_path() -> Result<PathBuf> {
    // Check for explicit config path from environment
    if let Ok(path) = std::env::var("ZELAN_CONFIG_PATH") {
        return Ok(PathBuf::from(path));
    }
    
    // For non-Docker environments, use the user's config directory
    if let Some(user_config_dir) = dirs_next::config_dir() {
        let config_dir = user_config_dir.join("zelan");
        std::fs::create_dir_all(&config_dir)?;
        return Ok(config_dir.join("config.json"));
    }
    
    // Fallback to current directory
    Ok(PathBuf::from("config.json"))
}

/// Load configuration from file or create default
async fn load_or_create_config(path: &Path) -> Result<Config> {
    // Check if file exists
    if !path.exists() {
        // Create default config
        let default_config = Config::default();
        save_config(path, &default_config).await?;
        info!("Created default configuration at {}", path.display());
        return Ok(default_config);
    }
    
    // Load existing config
    let config_str = fs::read_to_string(path).await?;
    let config: Config = serde_json::from_str(&config_str)?;
    debug!("Loaded configuration from {}", path.display());
    
    Ok(config)
}

/// Save configuration to file
async fn save_config(path: &Path, config: &Config) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    
    // Serialize and write
    let config_str = serde_json::to_string_pretty(config)?;
    fs::write(path, config_str).await?;
    debug!("Saved configuration to {}", path.display());
    
    Ok(())
}