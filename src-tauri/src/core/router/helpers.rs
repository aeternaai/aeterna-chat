use std::path::PathBuf;
use std::process::Stdio;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::process::{Child, Command};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::core::app::commands::get_jan_data_folder_path;
use super::models::RouterConfig;

/// Start the Python router service
///
/// # Arguments
/// * `app` - Tauri app handle
/// * `config` - Router configuration
///
/// # Returns
/// * `Ok(Child)` - Running child process
/// * `Err(String)` - Error message if failed to start
pub async fn start_router_service<R: Runtime>(
    app: &AppHandle<R>,
    config: &RouterConfig,
) -> Result<Child, String> {
    log::info!("Starting Python router service on {}:{}", config.host, config.port);

    // Get the router service directory
    let app_path = get_jan_data_folder_path(app.clone());
    let router_service_path = app_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or_else(|| "Failed to find router service path".to_string())?
        .join("router-service");

    if !router_service_path.exists() {
        return Err(format!(
            "Router service directory not found at: {}",
            router_service_path.display()
        ));
    }

    log::info!("Router service path: {}", router_service_path.display());

    // Determine Python executable
    let python_exe = if let Some(ref path) = config.python_path {
        path.clone()
    } else {
        // Try to find Python 3
        find_python_executable()?
    };

    log::info!("Using Python executable: {}", python_exe);

    // Build command
    let mut cmd = Command::new(&python_exe);
    cmd.arg("main.py")
        .arg("--host")
        .arg(&config.host)
        .arg("--port")
        .arg(config.port.to_string())
        .arg("--log-level")
        .arg(&config.log_level)
        .current_dir(&router_service_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    // Spawn the process
    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn router service: {}", e))?;

    log::info!("Router service started successfully");

    // Emit event
    let _ = app.emit("router-service-started", ());

    Ok(child)
}

/// Find Python executable on the system
fn find_python_executable() -> Result<String, String> {
    // Try common Python 3 commands in order
    let candidates = vec!["python3", "python"];

    for candidate in candidates {
        if which::which(candidate).is_ok() {
            log::info!("Found Python executable: {}", candidate);
            return Ok(candidate.to_string());
        }
    }

    Err("No Python executable found. Please install Python 3.11 or higher.".to_string())
}

/// Stop the router service
pub async fn stop_router_service(child: &mut Child) -> Result<(), String> {
    log::info!("Stopping router service...");

    // Try graceful shutdown first
    #[cfg(unix)]
    {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;

        if let Some(pid) = child.id() {
            let pid = Pid::from_raw(pid as i32);
            let _ = signal::kill(pid, Signal::SIGTERM);
            
            // Wait up to 5 seconds for graceful shutdown
            tokio::select! {
                _ = child.wait() => {
                    log::info!("Router service stopped gracefully");
                    return Ok(());
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                    log::warn!("Router service did not stop gracefully, killing...");
                }
            }
        }
    }

    // Force kill if graceful shutdown failed
    child
        .kill()
        .await
        .map_err(|e| format!("Failed to kill router service: {}", e))?;

    log::info!("Router service stopped");
    Ok(())
}

/// Check if router service is healthy
pub async fn check_router_health(config: &RouterConfig) -> Result<(), String> {
    let url = format!("http://{}:{}/health", config.host, config.port);
    
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .map_err(|e| format!("Health check failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Health check returned status: {}", response.status()));
    }

    Ok(())
}

/// Wait for router service to be ready
pub async fn wait_for_router_ready(
    config: &RouterConfig,
    max_attempts: u32,
) -> Result<(), String> {
    for attempt in 1..=max_attempts {
        log::debug!("Waiting for router service... (attempt {}/{})", attempt, max_attempts);
        
        if check_router_health(config).await.is_ok() {
            log::info!("Router service is ready");
            return Ok(());
        }

        if attempt < max_attempts {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    Err("Router service did not become ready in time".to_string())
}
