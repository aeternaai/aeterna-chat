use std::process::Stdio;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::process::{Child, Command};

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

    // Check if port is already in use
    if is_port_in_use(config.port).await {
        log::warn!("⚠️  Port {} is already in use by another process", config.port);
        log::warn!("   Attempting to find and kill conflicting process...");
        
        if let Err(e) = kill_process_on_port(config.port).await {
            return Err(format!(
                "Port {} is in use and could not free it: {}. Please run: lsof -ti :{} | xargs kill",
                config.port, e, config.port
            ));
        }
        
        log::info!("✅ Port {} is now available", config.port);
    }

    // Get the router service directory
    // In dev mode, use current_dir (project root)
    // In production, use resource_dir
    let router_service_path = if cfg!(dev) {
        std::env::current_dir()
            .map_err(|e| format!("Failed to get current dir: {}", e))?
            .join("router-service")
    } else {
        app.path()
            .resource_dir()
            .map_err(|e| format!("Failed to get resource dir: {}", e))?
            .join("router-service")
    };

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

    log::info!("Spawning Python router service...");
    log::info!("  Command: {} main.py --host {} --port {} --log-level {}", 
        python_exe, config.host, config.port, config.log_level);
    log::info!("  Working directory: {}", router_service_path.display());

    // Spawn the process
    let mut child = cmd
        .spawn()
        .map_err(|e| {
            let error_msg = format!("Failed to spawn router service process: {}", e);
            log::error!("{}", error_msg);
            error_msg
        })?;

    // Capture initial output to check for immediate errors
    let app_handle = app.clone();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Spawn a task to monitor stdout
    if let Some(stdout) = stdout {
        tauri::async_runtime::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                log::info!("[Python Router STDOUT] {}", line);
            }
        });
    }

    // Spawn a task to monitor stderr
    if let Some(stderr) = stderr {
        tauri::async_runtime::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                // Check for common Python errors
                if line.contains("ModuleNotFoundError") || 
                   line.contains("ImportError") ||
                   line.contains("Error") ||
                   line.contains("Exception") {
                    log::error!("[Python Router ERROR] {}", line);
                } else {
                    log::warn!("[Python Router STDERR] {}", line);
                }
            }
        });
    }

    log::info!("✅ Router service process spawned successfully (PID will be assigned by OS)");

    // Emit event
    let _ = app_handle.emit("router-service-started", ());

    Ok(child)
}

/// Find Python executable on the system
fn find_python_executable() -> Result<String, String> {
    log::info!("🔍 Searching for Python executable...");
    
    // Try common Python 3 commands in order
    let candidates = vec!["python3", "python"];

    for candidate in &candidates {
        log::debug!("   Checking for '{}'...", candidate);
        if let Ok(path) = which::which(candidate) {
            log::info!("✅ Found Python executable: {} ({})", candidate, path.display());
            return Ok(candidate.to_string());
        }
    }

    log::error!("❌ No Python executable found!");
    log::error!("   Tried: python3, python");
    log::error!("   Please install Python 3.11 or higher");
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
        .map_err(|e| {
            // Provide more specific error messages
            if e.is_timeout() {
                "Health check timed out (service may be starting or overloaded)".to_string()
            } else if e.is_connect() {
                format!("Cannot connect to service (is it running on port {}?)", config.port)
            } else {
                format!("Health check request failed: {}", e)
            }
        })?;

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
    log::info!("⏳ Waiting for router service to be ready on {}:{}...", config.host, config.port);
    log::info!("   Will check health endpoint {} times with 500ms intervals", max_attempts);
    
    for attempt in 1..=max_attempts {
        if attempt > 1 {
            log::debug!("⏳ Health check attempt {}/{}", attempt, max_attempts);
        }
        
        match check_router_health(config).await {
            Ok(_) => {
                log::info!("✅ Router service is healthy and ready! (attempt {}/{})", attempt, max_attempts);
                return Ok(());
            }
            Err(e) => {
                if attempt == max_attempts {
                    let error_msg = format!(
                        "❌ Router service failed to become ready after {} attempts ({}s total). Last error: {}",
                        max_attempts,
                        max_attempts as f32 * 0.5,
                        e
                    );
                    log::error!("{}", error_msg);
                    log::error!("💡 Possible causes:");
                    log::error!("   1. Python dependencies not installed (run: cd router-service && pip3 install -r requirements.txt)");
                    log::error!("   2. Port {} already in use", config.port);
                    log::error!("   3. Python process crashed - check stderr logs above");
                    return Err(error_msg);
                } else if attempt % 5 == 0 {
                    // Log every 5th attempt to avoid spam
                    log::debug!("   Still waiting... (attempt {}/{}, last error: {})", attempt, max_attempts, e);
                }
            }
        }

        if attempt < max_attempts {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    Err("Router service did not become ready in time".to_string())
}

/// Check if a port is in use
async fn is_port_in_use(port: u16) -> bool {
    use std::net::TcpListener;
    
    // Try to bind to the port
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_err()
}

/// Kill process using the specified port
async fn kill_process_on_port(port: u16) -> Result<(), String> {
    #[cfg(unix)]
    {
        use tokio::process::Command;
        
        // Find PIDs using the port
        let output = Command::new("lsof")
            .args(&["-ti", &format!(":{}", port)])
            .output()
            .await
            .map_err(|e| format!("Failed to run lsof: {}", e))?;
        
        if !output.status.success() {
            return Err("No process found on port".to_string());
        }
        
        let pids = String::from_utf8_lossy(&output.stdout);
        let pids: Vec<&str> = pids.trim().lines().collect();
        
        if pids.is_empty() {
            return Err("No process found on port".to_string());
        }
        
        log::info!("   Found {} process(es) using port {}: {:?}", pids.len(), port, pids);
        
        // Kill each PID
        for pid in pids {
            let kill_result = Command::new("kill")
                .arg(pid)
                .output()
                .await
                .map_err(|e| format!("Failed to kill process {}: {}", pid, e))?;
            
            if kill_result.status.success() {
                log::info!("   ✅ Killed process {}", pid);
            } else {
                log::warn!("   ⚠️  Failed to kill process {}", pid);
            }
        }
        
        // Wait a moment for the port to be freed
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        Ok(())
    }
    
    #[cfg(not(unix))]
    {
        Err("Port cleanup not implemented for this platform".to_string())
    }
}
