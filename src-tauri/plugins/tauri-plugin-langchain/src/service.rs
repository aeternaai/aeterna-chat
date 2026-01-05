//! LangChain service management - spawning, communication, lifecycle

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::Command;
use tokio::time::timeout;

use crate::error::{LangChainError, LangChainResult};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::state::{LangChainState, ServiceHandle, ServiceInfo};

/// Find the LangChain service binary path
/// 
/// Searches in order:
/// 1. Explicit path if provided
/// 2. Resources directory (bundled with app)
/// 3. User data directory
pub fn find_binary_path(explicit_path: Option<&str>) -> LangChainResult<PathBuf> {
    // Check explicit path first
    if let Some(path) = explicit_path {
        let pb = PathBuf::from(path);
        if pb.exists() {
            log::info!("Using explicit binary path: {:?}", pb);
            return Ok(pb);
        }
    }

    // Check resources directory (bundled with app)
    if let Some(resource_dir) = dirs::data_dir() {
        let bundled_path = resource_dir
            .join("Jan")
            .join("data")
            .join("langchain")
            .join("langchain-service");
        
        #[cfg(target_os = "windows")]
        let bundled_path = bundled_path.with_extension("exe");
        
        if bundled_path.exists() {
            log::info!("Using bundled binary: {:?}", bundled_path);
            return Ok(bundled_path);
        }
    }

    // Check for development mode - Python script directly
    // This allows running without PyInstaller during development
    let dev_script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("python")
        .join("langchain_service.py");
    
    if dev_script.exists() {
        log::info!("Using development Python script: {:?}", dev_script);
        return Ok(dev_script);
    }

    Err(LangChainError::BinaryNotFound(
        "Could not find langchain-service binary. Please ensure it's installed.".to_string()
    ))
}

/// Check if a path is a Python script (for dev mode)
fn is_python_script(path: &PathBuf) -> bool {
    path.extension().map_or(false, |ext| ext == "py")
}

/// Spawn the LangChain service subprocess
pub async fn spawn_service(
    state: &LangChainState,
    binary_path: Option<&str>,
    env_vars: Option<std::collections::HashMap<String, String>>,
) -> LangChainResult<ServiceInfo> {
    // Check if already running
    {
        let service = state.service.lock().await;
        if service.is_some() {
            return Err(LangChainError::Internal(
                "Service is already running".to_string()
            ));
        }
    }

    // Find binary
    let binary = find_binary_path(binary_path)?;
    log::info!("Spawning LangChain service from: {:?}", binary);

    // Determine if we need to run through Python interpreter
    let (program, args): (String, Vec<String>) = if is_python_script(&binary) {
        // Development mode - run with Python
        let python = find_python_interpreter()?;
        (python, vec![binary.to_string_lossy().to_string()])
    } else {
        // Production mode - run compiled binary directly
        (binary.to_string_lossy().to_string(), vec![])
    };

    // Build command
    let mut command = Command::new(&program);
    command.args(&args);
    
    // Set environment variables
    if let Some(envs) = env_vars {
        command.envs(envs);
    }

    // Configure stdio
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    // Spawn process
    let mut child = command.spawn().map_err(|e| {
        LangChainError::SpawnFailed(format!("Failed to spawn process: {}", e))
    })?;

    let pid = child.id().unwrap_or(0);
    log::info!("LangChain service spawned with PID: {}", pid);

    // Get stdio handles
    let stdin = child.stdin.take()
        .ok_or_else(|| LangChainError::SpawnFailed("Failed to capture stdin".to_string()))?;
    let stdout = child.stdout.take()
        .ok_or_else(|| LangChainError::SpawnFailed("Failed to capture stdout".to_string()))?;
    let stderr = child.stderr.take();

    // Spawn stderr logging task
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            log::info!("[LangChain stderr] {}", trimmed);
                        }
                    }
                    Err(e) => {
                        log::error!("Error reading LangChain stderr: {}", e);
                        break;
                    }
                }
            }
        });
    }

    // Create service info
    let info = ServiceInfo {
        pid,
        binary_path: binary.to_string_lossy().to_string(),
        started_at: chrono::Utc::now().timestamp(),
        is_healthy: false, // Will be updated after health check
        last_health_check: None,
        request_count: 0,
        error_count: 0,
    };

    // Create service handle
    let handle = ServiceHandle {
        child,
        stdin: BufWriter::new(stdin),
        stdout: BufReader::new(stdout),
        info: info.clone(),
    };

    // Store in state
    {
        let mut service = state.service.lock().await;
        *service = Some(handle);
    }

    // Store binary path
    {
        let mut path = state.binary_path.lock().await;
        *path = Some(binary.to_string_lossy().to_string());
    }

    // Wait a moment for service to initialize
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Perform initial health check
    match perform_health_check(state).await {
        Ok(_) => {
            state.set_health_status(true).await;
            log::info!("LangChain service is healthy and ready");
        }
        Err(e) => {
            log::warn!("Initial health check failed (service may still be starting): {}", e);
            // Don't fail here - service might just need more time
        }
    }

    Ok(info)
}

/// Find Python interpreter for development mode
fn find_python_interpreter() -> LangChainResult<String> {
    // Try common Python paths
    let candidates = ["python3", "python", "/usr/bin/python3", "/usr/local/bin/python3"];
    
    for candidate in candidates {
        if std::process::Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok()
        {
            return Ok(candidate.to_string());
        }
    }

    Err(LangChainError::BinaryNotFound(
        "Could not find Python interpreter. Please install Python 3.".to_string()
    ))
}

/// Shutdown the LangChain service
pub async fn shutdown_service(state: &LangChainState) -> LangChainResult<()> {
    let mut service = state.service.lock().await;
    
    if let Some(mut handle) = service.take() {
        log::info!("Shutting down LangChain service (PID: {})", handle.info.pid);

        // Send shutdown request
        let shutdown_request = crate::protocol::requests::shutdown();
        if let Err(e) = send_request_internal(&mut handle, &shutdown_request).await {
            log::warn!("Failed to send shutdown request: {}", e);
        }

        // Give service time to shutdown gracefully
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Kill if still running
        if let Err(e) = handle.child.kill().await {
            log::warn!("Failed to kill LangChain process: {}", e);
        }

        // Wait for process to exit
        let _ = handle.child.wait().await;

        log::info!("LangChain service shutdown complete");
    } else {
        log::warn!("No service running to shutdown");
    }

    Ok(())
}

/// Perform a health check on the service
pub async fn perform_health_check(state: &LangChainState) -> LangChainResult<crate::protocol::responses::HealthResponse> {
    let request = crate::protocol::requests::health();
    let response = send_request(state, request).await?;
    
    let health: crate::protocol::responses::HealthResponse = serde_json::from_value(response)
        .map_err(|e| LangChainError::IpcError(format!("Invalid health response: {}", e)))?;
    
    state.set_health_status(true).await;
    Ok(health)
}

/// Send a JSON-RPC request to the service
pub async fn send_request(
    state: &LangChainState,
    request: JsonRpcRequest,
) -> LangChainResult<serde_json::Value> {
    let config = state.config.lock().await.clone();
    
    let mut service = state.service.lock().await;
    let handle = service.as_mut()
        .ok_or_else(|| LangChainError::ServiceNotRunning("Service not started".to_string()))?;

    // Increment request count
    handle.info.request_count += 1;

    // Send with timeout
    let result = timeout(
        Duration::from_millis(config.call_timeout_ms),
        send_request_internal(handle, &request)
    ).await;

    match result {
        Ok(Ok(response)) => {
            match response.into_result() {
                Ok(value) => Ok(value),
                Err(rpc_err) => {
                    handle.info.error_count += 1;
                    Err(LangChainError::JsonRpcError {
                        code: rpc_err.code,
                        message: rpc_err.message,
                    })
                }
            }
        }
        Ok(Err(e)) => {
            handle.info.error_count += 1;
            handle.info.is_healthy = false;
            Err(e)
        }
        Err(_) => {
            handle.info.error_count += 1;
            handle.info.is_healthy = false;
            Err(LangChainError::Timeout(format!(
                "Request timed out after {}ms",
                config.call_timeout_ms
            )))
        }
    }
}

/// Internal function to send request without state locking
async fn send_request_internal(
    handle: &mut ServiceHandle,
    request: &JsonRpcRequest,
) -> LangChainResult<JsonRpcResponse> {
    // Serialize request
    let request_json = serde_json::to_string(request)?;
    log::debug!("Sending JSON-RPC request: {}", request_json);

    // Write request + newline
    handle.stdin.write_all(request_json.as_bytes()).await?;
    handle.stdin.write_all(b"\n").await?;
    handle.stdin.flush().await?;

    // Read response line
    let mut response_line = String::new();
    handle.stdout.read_line(&mut response_line).await?;

    if response_line.is_empty() {
        return Err(LangChainError::IpcError(
            "Empty response from service (process may have died)".to_string()
        ));
    }

    log::debug!("Received JSON-RPC response: {}", response_line.trim());

    // Parse response
    let response: JsonRpcResponse = serde_json::from_str(&response_line)?;

    // Validate response ID matches
    if response.id != request.id {
        return Err(LangChainError::IpcError(format!(
            "Response ID mismatch: expected {}, got {}",
            request.id, response.id
        )));
    }

    Ok(response)
}
