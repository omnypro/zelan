use tauri::State;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::AppState;
use crate::auth::providers::{AuthenticationFlow, AuthFlowStatus};
use crate::auth::token::{AuthState, TokenData};

/// Initiate authentication for a provider
#[tauri::command]
pub async fn start_authentication(
    provider: String,
    state: State<'_, AppState>,
) -> Result<AuthenticationFlow, String> {
    info!(provider = %provider, "Starting authentication for provider");
    
    state
        .auth_service
        .authenticate(&provider)
        .await
        .map_err(|e| e.to_string())
}

/// Check the status of an authentication flow
#[tauri::command]
pub async fn check_auth_flow_status(
    flow_id: String,
    state: State<'_, AppState>,
) -> Result<AuthFlowStatus, String> {
    state
        .auth_service
        .check_auth_flow(&flow_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get authentication state for a provider
#[tauri::command]
pub async fn get_auth_state(
    provider: String,
    state: State<'_, AppState>,
) -> Result<AuthState, String> {
    state
        .auth_service
        .get_auth_state(&provider)
        .await
        .map_err(|e| e.to_string())
}

/// Get a token for a provider (if authenticated)
#[tauri::command]
pub async fn get_auth_token(
    provider: String,
    state: State<'_, AppState>,
) -> Result<Option<TokenData>, String> {
    state
        .auth_service
        .get_token(&provider)
        .await
        .map_err(|e| e.to_string())
}

/// Clear authentication for a provider
#[tauri::command]
pub async fn clear_auth_tokens(
    provider: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!(provider = %provider, "Clearing auth tokens for provider");
    
    state
        .auth_service
        .clear_tokens(&provider)
        .await
        .map_err(|e| e.to_string())
}

/// Get available providers
#[tauri::command]
pub async fn get_available_auth_providers(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    // Currently hardcoded, but could be made dynamic in the future
    Ok(vec!["twitch".to_string()])
}