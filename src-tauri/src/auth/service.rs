use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use tauri::AppHandle;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::auth::providers::{AuthenticationFlow, AuthFlowStatus, AuthProvider, ScheduledRefresh};
use crate::auth::storage::SecureTokenStore;
use crate::auth::token::{AuthEvent, AuthState, TokenData};
use crate::events::{EventStream, Subscriber};

// Proactive refresh threshold
const PROACTIVE_REFRESH_THRESHOLD_SECS: i64 = 900; // 15 minutes
const TOKEN_EXPIRY_WARNING_THRESHOLD_SECS: i64 = 600; // 10 minutes
const TOKEN_MAINTENANCE_INTERVAL_SECS: u64 = 300; // 5 minutes
const REFRESH_TOKEN_EXPIRY_WARNING_DAYS: i64 = 5; // 5 days before 30-day expiry

/// Unified authentication service for all providers
pub struct AuthService {
    /// Map of provider name to provider implementation
    providers: HashMap<String, Box<dyn AuthProvider>>,
    /// In-memory token store
    memory_token_store: Arc<RwLock<HashMap<String, TokenData>>>,
    /// Secure token store for persistence
    secure_token_store: Arc<SecureTokenStore>,
    /// Event stream for auth events
    events: Arc<EventStream<AuthEvent>>,
    /// Queue of scheduled token refreshes
    refresh_queue: Arc<RwLock<VecDeque<ScheduledRefresh>>>,
    /// Map of provider name to current auth state
    auth_states: Arc<RwLock<HashMap<String, AuthState>>>,
    /// Active auth flows
    active_flows: Arc<RwLock<HashMap<String, String>>>, // Maps flow_id to provider
    /// Background maintenance task handle
    maintenance_task: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl AuthService {
    /// Create a new auth service
    pub fn new(secure_token_store: Arc<SecureTokenStore>) -> Self {
        let events = Arc::new(EventStream::new(100, 50));
        
        Self {
            providers: HashMap::new(),
            memory_token_store: Arc::new(RwLock::new(HashMap::new())),
            secure_token_store,
            events,
            refresh_queue: Arc::new(RwLock::new(VecDeque::new())),
            auth_states: Arc::new(RwLock::new(HashMap::new())),
            active_flows: Arc::new(RwLock::new(HashMap::new())),
            maintenance_task: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Register a provider with the service
    pub fn register_provider<P: AuthProvider + 'static>(&mut self, provider: P) {
        let provider_name = provider.name().to_string();
        info!(provider = %provider_name, "Registering auth provider");
        
        // Store the provider
        self.providers.insert(provider_name.clone(), Box::new(provider));
        
        // Initialize state as unauthenticated
        tokio::spawn({
            let auth_states = Arc::clone(&self.auth_states);
            let provider_name = provider_name.clone();
            async move {
                auth_states.write().await.insert(provider_name.clone(), AuthState::Unauthenticated);
            }
        });
    }
    
    /// Initialize the service and restore tokens from storage
    pub async fn initialize(&self, app_handle: Option<Arc<AppHandle>>) -> Result<()> {
        info!("Initializing auth service");
        
        // Load tokens for all registered providers
        for provider_name in self.providers.keys() {
            match self.restore_token_from_storage(provider_name).await {
                Ok(Some(token)) => {
                    info!(provider = %provider_name, "Restored token from secure storage");
                    
                    // Update auth state
                    self.update_auth_state(provider_name, AuthState::Authenticated { token }).await?;
                }
                Ok(None) => {
                    debug!(provider = %provider_name, "No token found in secure storage");
                }
                Err(e) => {
                    error!(
                        provider = %provider_name,
                        error = %e,
                        "Failed to restore token from secure storage"
                    );
                    
                    // Update auth state to failed
                    self.update_auth_state(
                        provider_name,
                        AuthState::Failed {
                            reason: format!("Failed to restore token: {}", e),
                            recoverable: true,
                        },
                    ).await?;
                }
            }
        }
        
        // Start token maintenance
        self.start_token_maintenance().await;
        
        Ok(())
    }
    
    /// Start periodic token maintenance
    async fn start_token_maintenance(&self) {
        let providers_clone = self.providers.keys().cloned().collect::<Vec<_>>();
        let memory_token_store = Arc::clone(&self.memory_token_store);
        let refresh_queue = Arc::clone(&self.refresh_queue);
        let auth_states = Arc::clone(&self.auth_states);
        let events = Arc::clone(&self.events);
        let secure_token_store = Arc::clone(&self.secure_token_store);
        
        // Create a weak reference to allow the task to be dropped
        let maintenance_task = Arc::clone(&self.maintenance_task);
        
        // Spawn a task to periodically check and maintain tokens
        let handle = tokio::spawn(async move {
            loop {
                // Check all providers
                for provider_name in &providers_clone {
                    // Skip if the provider is not authenticated
                    let current_state = {
                        let states = auth_states.read().await;
                        match states.get(provider_name) {
                            Some(state) => state.clone(),
                            None => continue,
                        }
                    };
                    
                    // Only perform maintenance on authenticated providers
                    if let AuthState::Authenticated { token } = current_state {
                        // Check if the token is about to expire
                        if let Some(expires_in) = token.seconds_until_expiration() {
                            if expires_in < TOKEN_EXPIRY_WARNING_THRESHOLD_SECS && expires_in > 0 {
                                // Emit warning event
                                let _ = events.publish(AuthEvent::TokenExpiringWarning {
                                    provider: provider_name.clone(),
                                    expires_in_secs: expires_in as u64,
                                }).await;
                                
                                // Schedule a refresh if expiring soon
                                if expires_in < PROACTIVE_REFRESH_THRESHOLD_SECS && expires_in > 0 {
                                    let refresh_time = Utc::now() + chrono::Duration::seconds(0); // Immediate
                                    refresh_queue.write().await.push_back(
                                        ScheduledRefresh::new(provider_name, refresh_time)
                                    );
                                    debug!(
                                        provider = %provider_name,
                                        expires_in_secs = expires_in,
                                        "Token approaching expiration, scheduled refresh"
                                    );
                                }
                            }
                            
                            // Check if refresh token is approaching 30-day expiry
                            if token.refresh_token_expiring_soon(REFRESH_TOKEN_EXPIRY_WARNING_DAYS) {
                                warn!(
                                    provider = %provider_name,
                                    "Refresh token approaching 30-day expiry, should re-authenticate"
                                );
                                
                                // Emit user action required event
                                let _ = events.publish(AuthEvent::UserActionRequired {
                                    provider: provider_name.clone(),
                                    action: "Your authentication will expire soon. Please re-authenticate to continue using this service.".to_string(),
                                }).await;
                            }
                        }
                    }
                }
                
                // Process the refresh queue
                let refresh_now = {
                    let mut queue = refresh_queue.write().await;
                    let mut to_process = Vec::new();
                    
                    // Get all due refreshes
                    while let Some(refresh) = queue.front() {
                        if refresh.is_due() {
                            to_process.push(queue.pop_front().unwrap());
                        } else {
                            break;
                        }
                    }
                    
                    to_process
                };
                
                // Process all due refreshes
                for refresh in refresh_now {
                    debug!(
                        provider = %refresh.provider,
                        "Processing scheduled token refresh"
                    );
                    
                    // Get current token
                    let current_token = {
                        let tokens = memory_token_store.read().await;
                        match tokens.get(&refresh.provider) {
                            Some(token) => token.clone(),
                            None => continue, // No token available
                        }
                    };
                    
                    // Update state to refreshing
                    let _ = auth_states.write().await.insert(
                        refresh.provider.clone(),
                        AuthState::Refreshing {
                            previous_token: current_token.clone(),
                        },
                    );
                    
                    // Emit state change event
                    let _ = events.publish(AuthEvent::StateChanged {
                        provider: refresh.provider.clone(),
                        state: AuthState::Refreshing {
                            previous_token: current_token.clone(),
                        },
                    }).await;
                    
                    // Attempt to refresh the token
                    // We have to clone the provider name because we need it for the match after
                    let provider_name = refresh.provider.clone();
                    match Self::refresh_token_for_provider(
                        &provider_name,
                        current_token,
                        memory_token_store.clone(),
                        secure_token_store.clone(),
                        events.clone(),
                        auth_states.clone(),
                    ).await {
                        Ok(_) => {
                            debug!(provider = %provider_name, "Token refresh completed successfully");
                        }
                        Err(e) => {
                            error!(
                                provider = %provider_name,
                                error = %e,
                                "Failed to refresh token"
                            );
                            
                            // Update state to failed
                            auth_states.write().await.insert(
                                provider_name.clone(),
                                AuthState::Failed {
                                    reason: format!("Failed to refresh token: {}", e),
                                    recoverable: true,
                                },
                            );
                            
                            // Emit state change event
                            let _ = events.publish(AuthEvent::StateChanged {
                                provider: provider_name.clone(),
                                state: AuthState::Failed {
                                    reason: format!("Failed to refresh token: {}", e),
                                    recoverable: true,
                                },
                            }).await;
                        }
                    }
                }
                
                // Sleep for maintenance interval
                sleep(Duration::from_secs(TOKEN_MAINTENANCE_INTERVAL_SECS)).await;
            }
        });
        
        // Store the task handle
        *self.maintenance_task.write().await = Some(handle);
    }
    
    /// Helper method to refresh a token
    async fn refresh_token_for_provider(
        provider_name: &str,
        token: TokenData,
        memory_token_store: Arc<RwLock<HashMap<String, TokenData>>>,
        secure_token_store: Arc<SecureTokenStore>,
        events: Arc<EventStream<AuthEvent>>,
        auth_states: Arc<RwLock<HashMap<String, AuthState>>>,
    ) -> Result<TokenData> {
        // Look up the provider to refresh the token
        // We can't store a reference to providers because we need to be 'static for tokio::spawn
        // This makes handling complex, so we'll create a custom error if we can't find it
        let provider_impl = match auth_states.read().await.get(provider_name) {
            Some(AuthState::Refreshing { previous_token }) => {
                if let Some(provider) = Self::get_provider_clone_from_name(provider_name) {
                    provider
                } else {
                    return Err(anyhow!("Provider not found: {}", provider_name));
                }
            }
            _ => return Err(anyhow!("Provider not in refreshing state: {}", provider_name)),
        };
        
        // Attempt to refresh the token
        let refreshed_token = provider_impl.refresh_token(token).await?;
        
        // Store the refreshed token
        memory_token_store.write().await.insert(provider_name.to_string(), refreshed_token.clone());
        
        // Store in secure storage
        if let Err(e) = secure_token_store.store(provider_name, &refreshed_token).await {
            warn!(
                provider = %provider_name,
                error = %e,
                "Failed to store refreshed token in secure storage"
            );
        }
        
        // Update state to authenticated
        auth_states.write().await.insert(
            provider_name.to_string(),
            AuthState::Authenticated {
                token: refreshed_token.clone(),
            },
        );
        
        // Emit state change event
        let _ = events.publish(AuthEvent::StateChanged {
            provider: provider_name.to_string(),
            state: AuthState::Authenticated {
                token: refreshed_token.clone(),
            },
        }).await;
        
        // Emit token refreshed event
        let _ = events.publish(AuthEvent::TokenRefreshed {
            provider: provider_name.to_string(),
            token: refreshed_token.clone(),
        }).await;
        
        Ok(refreshed_token)
    }
    
    /// Helper method to get a new instance of a provider by name (for use in async contexts)
    fn get_provider_clone_from_name(provider_name: &str) -> Option<Box<dyn AuthProvider>> {
        match provider_name {
            "twitch" => Some(Box::new(crate::auth::providers::twitch::TwitchAuthProvider::new())),
            // Add other providers here as they are implemented
            _ => None,
        }
    }
    
    /// Start the authentication process for a provider
    pub async fn authenticate(&self, provider_name: &str) -> Result<AuthenticationFlow> {
        let provider = self.get_provider(provider_name)?;
        
        // Update state to authenticating
        let flow_id = uuid::Uuid::new_v4().to_string();
        self.update_auth_state(
            provider_name,
            AuthState::Authenticating {
                flow_id: flow_id.clone(),
            },
        ).await?;
        
        // Start authentication flow
        let flow = provider.authenticate().await?;
        
        // Store the flow ID mapping
        self.active_flows.write().await.insert(flow_id.clone(), provider_name.to_string());
        
        // Return the flow to the caller
        Ok(flow)
    }
    
    /// Check the status of an authentication flow
    pub async fn check_auth_flow(&self, flow_id: &str) -> Result<AuthFlowStatus> {
        // Look up the provider for this flow
        let provider_name = {
            let flows = self.active_flows.read().await;
            match flows.get(flow_id) {
                Some(provider) => provider.clone(),
                None => return Err(anyhow!("No active flow with ID {}", flow_id)),
            }
        };
        
        let provider = self.get_provider(&provider_name)?;
        
        // Check flow status
        let status = provider.check_flow_status(flow_id).await?;
        
        // Handle completed flows
        match &status {
            AuthFlowStatus::Completed { token } => {
                // Store the token
                self.store_token(&provider_name, token).await?;
                
                // Update auth state
                self.update_auth_state(
                    &provider_name,
                    AuthState::Authenticated {
                        token: token.clone(),
                    },
                ).await?;
                
                // Remove the flow
                self.active_flows.write().await.remove(flow_id);
            }
            AuthFlowStatus::Expired => {
                // Remove the flow and update state
                self.active_flows.write().await.remove(flow_id);
                self.update_auth_state(
                    &provider_name,
                    AuthState::Failed {
                        reason: "Authentication flow expired".to_string(),
                        recoverable: true,
                    },
                ).await?;
            }
            AuthFlowStatus::Failed { reason } => {
                // Remove the flow and update state
                self.active_flows.write().await.remove(flow_id);
                self.update_auth_state(
                    &provider_name,
                    AuthState::Failed {
                        reason: reason.clone(),
                        recoverable: true,
                    },
                ).await?;
            }
            _ => {} // No state change for pending
        }
        
        Ok(status)
    }
    
    /// Get a token for a provider
    pub async fn get_token(&self, provider_name: &str) -> Result<Option<TokenData>> {
        // Get the current state
        let state = self.get_auth_state(provider_name).await?;
        
        match state {
            AuthState::Authenticated { token } => Ok(Some(token)),
            _ => Ok(None),
        }
    }
    
    /// Get the current auth state for a provider
    pub async fn get_auth_state(&self, provider_name: &str) -> Result<AuthState> {
        let states = self.auth_states.read().await;
        match states.get(provider_name) {
            Some(state) => Ok(state.clone()),
            None => Err(anyhow!("No auth state for provider: {}", provider_name)),
        }
    }
    
    /// Update the auth state for a provider
    async fn update_auth_state(&self, provider_name: &str, state: AuthState) -> Result<()> {
        // Store the state
        self.auth_states.write().await.insert(provider_name.to_string(), state.clone());
        
        // Emit event
        self.events.publish(AuthEvent::StateChanged {
            provider: provider_name.to_string(),
            state,
        }).await?;
        
        Ok(())
    }
    
    /// Store a token in memory and secure storage
    async fn store_token(&self, provider_name: &str, token: &TokenData) -> Result<()> {
        // Store in memory
        self.memory_token_store.write().await.insert(provider_name.to_string(), token.clone());
        
        // Store in secure storage
        self.secure_token_store.store(provider_name, token).await?;
        
        Ok(())
    }
    
    /// Restore a token from secure storage
    async fn restore_token_from_storage(&self, provider_name: &str) -> Result<Option<TokenData>> {
        // Try to get from secure storage
        let token = self.secure_token_store.retrieve(provider_name).await?;
        
        // If found, validate and store in memory
        if let Some(token) = token {
            // Validate the token
            let provider = self.get_provider(provider_name)?;
            match provider.validate_token(token.clone()).await {
                Ok(validated_token) => {
                    // Store validated token in memory
                    self.memory_token_store.write().await.insert(
                        provider_name.to_string(),
                        validated_token.clone(),
                    );
                    Ok(Some(validated_token))
                }
                Err(e) => {
                    // Token is invalid, try to refresh
                    if let Some(refresh_token) = &token.refresh_token {
                        debug!(
                            provider = %provider_name,
                            error = %e,
                            "Token validation failed, attempting refresh"
                        );
                        
                        match provider.refresh_token(token).await {
                            Ok(refreshed_token) => {
                                // Store refreshed token
                                self.store_token(provider_name, &refreshed_token).await?;
                                Ok(Some(refreshed_token))
                            }
                            Err(refresh_err) => {
                                error!(
                                    provider = %provider_name,
                                    error = %refresh_err,
                                    "Failed to refresh invalid token"
                                );
                                
                                // Clear invalid token
                                self.secure_token_store.clear(provider_name).await?;
                                
                                Err(anyhow!(
                                    "Token validation and refresh failed: {}. Cleared invalid token.",
                                    refresh_err
                                ))
                            }
                        }
                    } else {
                        // No refresh token, can't recover
                        error!(
                            provider = %provider_name,
                            error = %e,
                            "Token validation failed and no refresh token available"
                        );
                        
                        // Clear invalid token
                        self.secure_token_store.clear(provider_name).await?;
                        
                        Err(anyhow!(
                            "Token validation failed and no refresh token available: {}. Cleared invalid token.",
                            e
                        ))
                    }
                }
            }
        } else {
            // No token found
            Ok(None)
        }
    }
    
    /// Clear tokens for a provider
    pub async fn clear_tokens(&self, provider_name: &str) -> Result<()> {
        // Clear from memory
        self.memory_token_store.write().await.remove(provider_name);
        
        // Clear from secure storage
        self.secure_token_store.clear(provider_name).await?;
        
        // Update auth state
        self.update_auth_state(provider_name, AuthState::Unauthenticated).await?;
        
        Ok(())
    }
    
    /// Get a provider by name
    fn get_provider(&self, provider_name: &str) -> Result<&Box<dyn AuthProvider>> {
        match self.providers.get(provider_name) {
            Some(provider) => Ok(provider),
            None => Err(anyhow!("Provider not found: {}", provider_name)),
        }
    }
    
    /// Schedule a token refresh
    pub async fn schedule_refresh(&self, provider_name: &str, when: DateTime<Utc>) -> Result<()> {
        // Verify provider exists
        self.get_provider(provider_name)?;
        
        // Add to refresh queue
        self.refresh_queue.write().await.push_back(
            ScheduledRefresh::new(provider_name, when)
        );
        
        // Emit event
        self.events.publish(AuthEvent::RefreshScheduled {
            provider: provider_name.to_string(),
            when,
        }).await?;
        
        Ok(())
    }
    
    /// Subscribe to auth events
    pub fn subscribe(&self) -> Subscriber<AuthEvent> {
        self.events.subscribe()
    }
}