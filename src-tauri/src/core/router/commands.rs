use tauri::{AppHandle, Runtime, State};
use tokio::time::timeout;
use std::time::Duration;

use crate::core::state::AppState;
use super::helpers::{start_router_service, stop_router_service, wait_for_router_ready};
use super::models::{RouteRequest, RouteResponse, RouterConfig, HealthResponse, StrategyInfo};

/// Start the router service
#[tauri::command]
pub async fn start_router<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("📋 Router service start requested");

    // Check if already running
    {
        let router_process = state.router_process.lock().await;
        if router_process.is_some() {
            log::warn!("⚠️  Router service is already running, skipping start");
            return Ok(());
        }
    }

    // Get config (use default for now, can be customized via settings)
    let config = RouterConfig::default();
    log::info!("📋 Router config: host={}, port={}", config.host, config.port);

    // Start the service
    log::info!("🚀 Starting router service process...");
    let child = start_router_service(&app, &config).await.map_err(|e| {
        let error_msg = format!("Failed to start router service: {}", e);
        log::error!("❌ {}", error_msg);
        error_msg
    })?;

    // Wait for service to be ready
    wait_for_router_ready(&config, 20).await.map_err(|e| {
        let error_msg = format!("Router service did not become ready: {}", e);
        log::error!("❌ {}", error_msg);
        error_msg
    })?;

    // Store the process handle
    {
        let mut router_process = state.router_process.lock().await;
        *router_process = Some(child);
    }

    // Store the config
    {
        let mut router_config = state.router_config.lock().await;
        *router_config = config;
    }

    log::info!("✅ Router service is fully operational");
    Ok(())
}

/// Stop the router service
#[tauri::command]
pub async fn stop_router(state: State<'_, AppState>) -> Result<(), String> {
    log::info!("Stopping router service...");

    let mut router_process = state.router_process.lock().await;

    if let Some(mut child) = router_process.take() {
        stop_router_service(&mut child).await?;
        log::info!("Router service stopped");
    } else {
        log::warn!("Router service was not running");
    }

    Ok(())
}

/// Route a request using the Python router service
#[tauri::command]
pub async fn route_request(
    state: State<'_, AppState>,
    request: RouteRequest,
) -> Result<RouteResponse, String> {
    log::debug!("Routing request with {} messages", request.messages.len());

    // Get config
    let config = state.router_config.lock().await.clone();

    // Make HTTP request to router service
    let url = format!("http://{}:{}/route", config.host, config.port);
    
    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(30),
        client.post(&url).json(&request).send()
    )
    .await
    .map_err(|_| "Router request timed out".to_string())?
    .map_err(|e| format!("Router request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Router returned error {}: {}", status, error_text));
    }

    let route_response: RouteResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse router response: {}", e))?;

    log::debug!(
        "Routed to model {} with confidence {:.2}",
        route_response.model_id,
        route_response.confidence
    );

    Ok(route_response)
}

/// Get router health status
#[tauri::command]
pub async fn get_router_health(state: State<'_, AppState>) -> Result<HealthResponse, String> {
    let config = state.router_config.lock().await.clone();
    let url = format!("http://{}:{}/health", config.host, config.port);

    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(5),
        client.get(&url).send()
    )
    .await
    .map_err(|_| "Health check timed out".to_string())?
    .map_err(|e| format!("Health check failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Health check returned status: {}", response.status()));
    }

    let health: HealthResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse health response: {}", e))?;

    Ok(health)
}

/// List available routing strategies
#[tauri::command]
pub async fn list_router_strategies(state: State<'_, AppState>) -> Result<Vec<StrategyInfo>, String> {
    let config = state.router_config.lock().await.clone();
    let url = format!("http://{}:{}/strategies", config.host, config.port);

    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(5),
        client.get(&url).send()
    )
    .await
    .map_err(|_| "Request timed out".to_string())?
    .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Request returned status: {}", response.status()));
    }

    let strategies: Vec<StrategyInfo> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(strategies)
}

/// Set the active routing strategy
#[tauri::command]
pub async fn set_router_strategy(
    state: State<'_, AppState>,
    strategy_name: String,
) -> Result<(), String> {
    let config = state.router_config.lock().await.clone();
    let url = format!("http://{}:{}/strategy/{}", config.host, config.port, strategy_name);

    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(5),
        client.post(&url).send()
    )
    .await
    .map_err(|_| "Request timed out".to_string())?
    .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Failed to set strategy: {}", error_text));
    }

    log::info!("Set router strategy to: {}", strategy_name);
    Ok(())
}
