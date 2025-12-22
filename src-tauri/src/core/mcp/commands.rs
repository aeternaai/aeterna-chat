use rmcp::model::{CallToolRequestParam, CallToolResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::sync::oneshot;
use tokio::time::timeout;

// Maximum tokens to cache (1K shown initially + 20K available for retrieval)
const MAX_CACHED_TOKENS: usize = 21_000;
// Maximum tokens per fetch_cached_output call (forces exploration in chunks)
const MAX_TOKENS_PER_FETCH: usize = 5_000;

use super::{
    constants::DEFAULT_MCP_CONFIG,
    helpers::{extract_command_args, restart_active_mcp_servers, start_mcp_server_with_restart, stop_mcp_servers},
};
use crate::core::{
    app::commands::get_jan_data_folder_path,
    mcp::models::McpSettings,
    state::AppState,
};
use crate::core::{
    mcp::models::ToolWithServer,
    state::{RunningServiceEnum, SharedMcpServers},
};
use std::{collections::HashMap, fs, time::Duration};

/// OAuth flow result containing the authorization URL
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlowResult {
    pub auth_url: String,
}

async fn tool_call_timeout(state: &State<'_, AppState>) -> Duration {
    state
        .mcp_settings
        .lock()
        .await
        .tool_call_timeout_duration()
}

#[tauri::command]
pub async fn activate_mcp_server<R: Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    name: String,
    config: Value,
) -> Result<(), String> {
    let servers: SharedMcpServers = state.mcp_servers.clone();

    // Use the modified start_mcp_server_with_restart that returns first attempt result
    start_mcp_server_with_restart(app, servers, name, config, Some(3)).await
}

#[tauri::command]
pub async fn deactivate_mcp_server<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    log::info!("Deactivating MCP server: {name}");

    // Get port from config before removing (for lock file cleanup later)
    let bridge_port = if name == "Jan Browser MCP" {
        let active_servers = state.mcp_active_servers.lock().await;
        active_servers.get(&name).and_then(|config| {
            config
                .get("envs")
                .and_then(|envs| envs.get("BRIDGE_PORT"))
                .and_then(|port| port.as_str())
                .and_then(|port_str| port_str.parse::<u16>().ok())
        })
    } else {
        None
    };

    // First, mark server as manually deactivated to prevent restart
    // Remove from active servers list to prevent restart
    {
        let mut active_servers = state.mcp_active_servers.lock().await;
        active_servers.remove(&name);
        log::info!("Removed MCP server {name} from active servers list");
    }

    // Mark as not successfully connected to prevent restart logic
    {
        let mut connected = state.mcp_successfully_connected.lock().await;
        connected.insert(name.clone(), false);
        log::info!("Marked MCP server {name} as not successfully connected");
    }

    // Reset restart count
    {
        let mut counts = state.mcp_restart_counts.lock().await;
        counts.remove(&name);
        log::info!("Reset restart count for MCP server {name}");
    }

    // Reset OAuth URL opened flag
    {
        let mut opened = state.mcp_oauth_url_opened.lock().await;
        opened.remove(&name);
        log::info!("Reset OAuth URL opened flag for MCP server {name}");
    }

    // Now remove and stop the server
    let servers = state.mcp_servers.clone();
    let mut servers_map = servers.lock().await;

    let service = servers_map
        .remove(&name)
        .ok_or_else(|| format!("Server {name} not found"))?;

    // Release the lock before calling cancel
    drop(servers_map);

    match service {
        RunningServiceEnum::NoInit(service) => {
            log::info!("Stopping server {name}...");
            service.cancel().await.map_err(|e| e.to_string())?;
        }
        RunningServiceEnum::WithInit(service) => {
            log::info!("Stopping server {name} with initialization...");
            service.cancel().await.map_err(|e| e.to_string())?;
        }
    }

    // Delete lock file if this is Jan Browser MCP and we have a port
    if name == "Jan Browser MCP" {
        if let Some(port) = bridge_port {
            use crate::core::mcp::lockfile::delete_lock_file;

            if let Err(e) = delete_lock_file(&app, port) {
                log::warn!("Failed to delete lock file for port {}: {}", port, e);
            }
        }
    }

    log::info!("Server {name} stopped successfully and marked as deactivated.");
    Ok(())
}

#[tauri::command]
pub async fn restart_mcp_servers<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> Result<(), String> {
    let servers = state.mcp_servers.clone();
    // Stop the servers
    stop_mcp_servers(state.mcp_servers.clone()).await?;

    // Restart only previously active servers (like cortex)
    restart_active_mcp_servers(&app, servers).await?;

    app.emit("mcp-update", "MCP servers updated")
        .map_err(|e| format!("Failed to emit event: {e}"))?;

    Ok(())
}

/// Reset MCP restart count for a specific server (like cortex reset)
#[tauri::command]
pub async fn reset_mcp_restart_count(
    state: State<'_, AppState>,
    server_name: String,
) -> Result<(), String> {
    let mut counts = state.mcp_restart_counts.lock().await;

    let count = match counts.get_mut(&server_name) {
        Some(count) => count,
        None => return Ok(()), // Server not found, nothing to reset
    };

    let old_count = *count;
    *count = 0;
    log::info!(
        "MCP server {server_name} restart count reset from {old_count} to 0."
    );
    Ok(())
}

#[tauri::command]
pub async fn get_connected_servers(
    _app: AppHandle<impl Runtime>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let servers = state.mcp_servers.clone();
    let servers_map = servers.lock().await;
    Ok(servers_map.keys().cloned().collect())
}

/// Retrieves all available tools from all MCP servers with server information
///
/// # Arguments
/// * `state` - Application state containing MCP server connections
///
/// # Returns
/// * `Result<Vec<Tool>, String>` - A vector of all tools if successful, or an error message if failed
///
/// This function:
/// 1. Locks the MCP servers mutex to access server connections
/// 2. Iterates through all connected servers
/// 3. Gets the list of tools from each server
/// 4. Associates each tool with its parent server name
/// 5. Combines all tools into a single vector
/// 6. Returns the combined list of all available tools with server information
#[tauri::command]
pub async fn get_tools(state: State<'_, AppState>) -> Result<Vec<ToolWithServer>, String> {
    let timeout_duration = tool_call_timeout(&state).await;
    let servers = state.mcp_servers.lock().await;
    let mut all_tools: Vec<ToolWithServer> = Vec::new();

    // Add virtual internal tool for cache retrieval
    all_tools.push(ToolWithServer {
        name: "fetch_cached_output".to_string(),
        description: Some(
            "Retrieve cached tool output by reference ID. The cache stores the first 21,000 tokens of large outputs. \
             CRITICAL RULES: \
             1. ALWAYS provide explicit numeric end_token (do NOT use 'end' or omit it) \
             2. Maximum 5,000 tokens per call (end_token - start_token ≤ 5000) \
             3. Make multiple calls with 5K chunks: (0-5000), (5000-10000), (10000-15000), (15000-20000) \
             4. If info not found in 21K tokens, state it was not found. \
             EXAMPLE CORRECT USAGE: fetch_cached_output(ref_id='...', start_token=0, end_token=5000)".to_string()
        ),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "ref_id": {
                    "type": "string",
                    "description": "The cache reference ID from the truncated output message"
                },
                "start_token": {
                    "type": "number",
                    "description": "Starting token index (REQUIRED, 0-based, range: 0-20999). Use 0 for first chunk."
                },
                "end_token": {
                    "type": "number",
                    "description": "Ending token index (REQUIRED, range: 1-21000). MUST be numeric (not 'end'). Range size MUST NOT exceed 5000: (end_token - start_token ≤ 5000). Use 5000 for first chunk."
                }
            },
            "required": ["ref_id", "start_token", "end_token"]
        }),
        server: "_internal".to_string(),
    });

    for (server_name, service) in servers.iter() {
        // List tools with timeout
        let tools_future = service.list_all_tools();
        let tools = match timeout(timeout_duration, tools_future).await {
            Ok(result) => result.map_err(|e| e.to_string())?,
            Err(_) => {
                log::warn!(
                    "Listing tools timed out after {} seconds",
                    timeout_duration.as_secs()
                );
                continue; // Skip this server and continue with others
            }
        };

        for tool in tools {
            all_tools.push(ToolWithServer {
                name: tool.name.to_string(),
                description: tool.description.as_ref().map(|d| d.to_string()),
                input_schema: serde_json::Value::Object((*tool.input_schema).clone()),
                server: server_name.clone(),
            });
        }
    }

    Ok(all_tools)
}

/// Calls a tool on an MCP server by name with optional arguments
///
/// # Arguments
/// * `state` - Application state containing MCP server connections
/// * `tool_name` - Name of the tool to call
/// * `server_name` - Optional name of the server to call the tool from (for disambiguation)
/// * `arguments` - Optional map of argument names to values
/// * `cancellation_token` - Optional token to allow cancellation from JS side
///
/// # Returns
/// * `Result<CallToolResult, String>` - Result of the tool call if successful, or error message if failed
///
/// This function:
/// 1. Locks the MCP servers mutex to access server connections
/// 2. If server_name is provided, looks for the tool in that specific server
/// 3. Otherwise, searches through all servers for one containing the named tool
/// 4. When found, calls the tool on that server with the provided arguments
/// 5. Supports cancellation via cancellation_token
/// 6. Returns error if no server has the requested tool or if specified server not found
#[tauri::command]
pub async fn call_tool(
    state: State<'_, AppState>,
    tool_name: String,
    server_name: Option<String>,
    arguments: Option<Map<String, Value>>,
    cancellation_token: Option<String>,
) -> Result<CallToolResult, String> {
    // Handle internal virtual tool for cache retrieval
    if tool_name == "fetch_cached_output" {
        log::info!("🔍 Model is calling fetch_cached_output virtual tool");
        
        let ref_id = arguments
            .as_ref()
            .and_then(|args| args.get("ref_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required parameter: ref_id".to_string())?;
        
        // Validate start_token is provided and numeric
        let start_token = arguments
            .as_ref()
            .and_then(|args| args.get("start_token"))
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .ok_or_else(|| {
                "❌ REJECTED: Missing or invalid 'start_token'. MUST provide numeric value (0-20999).\n\
                 CORRECT EXAMPLE: fetch_cached_output(ref_id='...', start_token=0, end_token=5000)".to_string()
            })?;
        
        // Validate end_token is provided and numeric (REQUIRED - no default)
        let end_token_requested = arguments
            .as_ref()
            .and_then(|args| args.get("end_token"))
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .ok_or_else(|| {
                format!(
                    "❌ REJECTED: Missing or invalid 'end_token'. You MUST provide explicit numeric end_token (not 'end' or null).\n\n\
                     BOTH start_token AND end_token are REQUIRED parameters.\n\n\
                     CORRECT EXAMPLE:\n  \
                     fetch_cached_output(ref_id='{}', start_token={}, end_token={})\n\n\
                     Then continue with:\n  \
                     fetch_cached_output(ref_id='{}', start_token={}, end_token={})",
                    ref_id,
                    start_token,
                    (start_token + MAX_TOKENS_PER_FETCH).min(MAX_CACHED_TOKENS),
                    ref_id,
                    (start_token + MAX_TOKENS_PER_FETCH).min(MAX_CACHED_TOKENS),
                    (start_token + MAX_TOKENS_PER_FETCH * 2).min(MAX_CACHED_TOKENS)
                )
            })?;
        
        log::info!(
            "📥 Model requesting cached content: ref_id='{}', token_range={}-{}",
            ref_id,
            start_token,
            end_token_requested
        );
        
        // Validate start_token is within bounds
        if start_token >= MAX_CACHED_TOKENS {
            log::warn!(
                "⚠️ Requested start_token {} exceeds cache limit {} - returning error",
                start_token,
                MAX_CACHED_TOKENS
            );
            return Ok(CallToolResult {
                content: vec![],
                structured_content: Some(json!({
                    "error": format!(
                        "❌ REJECTED: start_token {} exceeds cached limit (0-{}).\n\n\
                         CORRECT EXAMPLE:\n  \
                         fetch_cached_output(ref_id='{}', start_token=0, end_token=5000)",
                        start_token,
                        MAX_CACHED_TOKENS - 1,
                        ref_id
                    ),
                    "cache_limit": MAX_CACHED_TOKENS
                })),
                is_error: Some(true),
                meta: None,
            });
        }
        
        // Clamp end_token to cache limit if it exceeds
        let end_token = end_token_requested.min(MAX_CACHED_TOKENS);
        
        if end_token_requested > MAX_CACHED_TOKENS {
            log::warn!(
                "⚠️ Requested end_token {} exceeds cache limit {}, clamping to {}",
                end_token_requested,
                MAX_CACHED_TOKENS,
                end_token
            );
        }
        
        // Enforce maximum tokens per fetch (5K limit)
        let requested_range_size = end_token.saturating_sub(start_token);
        if requested_range_size > MAX_TOKENS_PER_FETCH {
            log::warn!(
                "🚫 Model requested {} tokens ({}-{}), exceeding per-fetch limit of {} tokens",
                requested_range_size,
                start_token,
                end_token,
                MAX_TOKENS_PER_FETCH
            );
            return Ok(CallToolResult {
                content: vec![],
                structured_content: Some(json!({
                    "error": format!(
                        "❌ REJECTED: Requested {} tokens ({}-{}) exceeds 5000 token per-call limit.\n\n\
                         You MUST use smaller ranges. DO NOT use 'end' or omit end_token.\n\n\
                         CORRECT EXAMPLE:\n  \
                         fetch_cached_output(ref_id='{}', start_token={}, end_token={})\n\n\
                         Then continue with:\n  \
                         fetch_cached_output(ref_id='{}', start_token={}, end_token={})",
                        requested_range_size,
                        start_token,
                        end_token,
                        ref_id,
                        start_token,
                        start_token + MAX_TOKENS_PER_FETCH,
                        ref_id,
                        start_token + MAX_TOKENS_PER_FETCH,
                        (start_token + MAX_TOKENS_PER_FETCH).min(MAX_CACHED_TOKENS) + MAX_TOKENS_PER_FETCH
                    ),
                    "requested_size": requested_range_size,
                    "max_per_fetch": MAX_TOKENS_PER_FETCH,
                    "correct_call_example": format!("fetch_cached_output(ref_id='{}', start_token={}, end_token={})", ref_id, start_token, start_token + MAX_TOKENS_PER_FETCH)
                })),
                is_error: Some(true),
                meta: None,
            });
        }
        
        log::info!(
            "🔄 Processing cache retrieval: {} tokens ({}-{}) | Per-fetch limit: {} | Cache limit: {}",
            requested_range_size,
            start_token,
            end_token,
            MAX_TOKENS_PER_FETCH,
            MAX_CACHED_TOKENS
        );
        
        // Retrieve from cache
        let cache = state.tool_output_cache.inner();
        let cache_map = cache.lock().await;
        
        if let Some(cached) = cache_map.get(ref_id) {
            let content_result = cached.get_range(start_token, end_token);
            
            let content = content_result.map_err(|e| {
                log::error!("❌ Cache retrieval failed: {}", e);
                e
            })?;
            
            let retrieved_tokens = content.len() / 4;
            let progress_pct = (end_token as f64 / MAX_CACHED_TOKENS as f64 * 100.0) as u32;
            let fetch_size = end_token - start_token;
            
            log::info!(
                "✅ Retrieved {} characters ({} tokens) | Range: {}-{} | Fetch size: {}/{} tokens | Progress: {}/{} ({}%)",
                content.len(),
                retrieved_tokens,
                start_token,
                end_token,
                fetch_size,
                MAX_TOKENS_PER_FETCH,
                end_token,
                MAX_CACHED_TOKENS,
                progress_pct
            );
            
            // Mark as ephemeral - frontend should not persist this in conversation history
            let mut meta_map = Map::new();
            meta_map.insert("ephemeral".to_string(), json!(true));
            meta_map.insert("is_cache_fetch".to_string(), json!(true));
            meta_map.insert("chunk_range".to_string(), json!(format!("{}-{}", start_token, end_token)));
            meta_map.insert("ref_id".to_string(), json!(ref_id));
            
            let result = CallToolResult {
                content: vec![],
                structured_content: Some(serde_json::json!({
                    "text": content,
                    "metadata": {
                        "ref_id": ref_id,
                        "token_range": format!("{}-{}", start_token, end_token),
                        "cache_limit": MAX_CACHED_TOKENS,
                        "retrieved_tokens": retrieved_tokens,
                        "progress_percent": progress_pct
                    }
                })),
                is_error: None,
                meta: Some(rmcp::model::Meta(meta_map.clone())),
            };
            
            // Comprehensive debug logging
            log::warn!("🚨 [EPHEMERAL] fetch_cached_output returning with ephemeral=true");
            log::warn!("🚨 [EPHEMERAL] meta field: {:?}", result.meta);
            log::warn!("🚨 [EPHEMERAL] meta_map contents: {:?}", meta_map);
            
            // Log the full JSON that will be sent to frontend
            if let Ok(json) = serde_json::to_string(&result) {
                log::warn!("🚨 [EPHEMERAL] Full JSON response:\n{}", json);
            }
            
            return Ok(result);
        } else {
            return Err(format!("Cache entry not found for ref_id: {}", ref_id));
        }
    }

    let timeout_duration = tool_call_timeout(&state).await;
    // Set up cancellation if token is provided
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

    if let Some(token) = &cancellation_token {
        let mut cancellations = state.tool_call_cancellations.lock().await;
        cancellations.insert(token.clone(), cancel_tx);
    }

    let servers = state.mcp_servers.lock().await;

    // If server_name is provided, only check that specific server
    let servers_to_check: Vec<(&String, &crate::core::state::RunningServiceEnum)> = if let Some(ref server) = server_name {
        servers.iter()
            .filter(|(name, _)| *name == server)
            .collect()
    } else {
        servers.iter().collect()
    };

    if servers_to_check.is_empty() {
        if let Some(server) = server_name {
            return Err(format!("Server '{server}' not found"));
        }
    }

    // Iterate through servers and find the one that contains the tool
    for (srv_name, service) in servers_to_check.iter() {
        let tools = match service.list_all_tools().await {
            Ok(tools) => tools,
            Err(_) => continue, // Skip this server if we can't list tools
        };

        if !tools.iter().any(|t| t.name == tool_name) {
            continue; // Tool not found in this server, try next
        }

        println!("Found tool {tool_name} in server {srv_name}");

        // Call the tool with timeout and cancellation support
        let tool_call = service.call_tool(CallToolRequestParam {
            name: tool_name.clone().into(),
            arguments,
        });

        // Race between timeout, tool call, and cancellation
        let result = if cancellation_token.is_some() {
            tokio::select! {
                result = timeout(timeout_duration, tool_call) => {
                    match result {
                        Ok(call_result) => call_result.map_err(|e| e.to_string()),
                        Err(_) => Err(format!(
                            "Tool call '{tool_name}' timed out after {} seconds",
                            timeout_duration.as_secs()
                        )),
                    }
                }
                _ = cancel_rx => {
                    Err(format!("Tool call '{tool_name}' was cancelled"))
                }
            }
        } else {
            match timeout(timeout_duration, tool_call).await {
                Ok(call_result) => call_result.map_err(|e| e.to_string()),
                Err(_) => Err(format!(
                    "Tool call '{tool_name}' timed out after {} seconds",
                    timeout_duration.as_secs()
                )),
            }
        };

        // Clean up cancellation token
        if let Some(token) = &cancellation_token {
            let mut cancellations = state.tool_call_cancellations.lock().await;
            cancellations.remove(token);
        }

        // Process result and apply caching for large outputs
        if let Ok(mut call_result) = result {
            // Serialize result to check size and potentially cache
            if let Ok(json_str) = serde_json::to_string(&call_result) {
                use crate::core::mcp::cache::{maybe_cache_output, estimate_tokens, MAX_TOKENS_IN_RESPONSE};
                
                let token_count = estimate_tokens(&json_str);
                
                // If output is large, cache it and return truncated version
                if token_count > MAX_TOKENS_IN_RESPONSE {
                    // Slice to MAX_CACHED_TOKENS before caching
                    let content_to_cache = if token_count > MAX_CACHED_TOKENS {
                        log::info!(
                            "📦 Slicing output from {} tokens to {} tokens (cache limit) for tool '{}'",
                            token_count,
                            MAX_CACHED_TOKENS,
                            tool_name
                        );
                        let max_chars = MAX_CACHED_TOKENS * 4;
                        if json_str.len() > max_chars {
                            json_str[..max_chars].to_string()
                        } else {
                            json_str.clone()
                        }
                    } else {
                        json_str.clone()
                    };
                    
                    if let Some(cached) = maybe_cache_output(
                        &state.tool_output_cache,
                        &content_to_cache,
                        &tool_name,
                        srv_name,
                        "application/json",
                    )
                    .await
                    {
                        log::warn!(
                            "Tool '{}' output is large ({} tokens). Cached with ref_id: '{}'. \
                             Returning truncated version. Full output available via fetch_cached_tool_output(ref_id: '{}', start_token, end_token)",
                            tool_name,
                            cached.total_tokens,
                            cached.ref_id,
                            cached.ref_id
                        );
                        
                        // Return truncated version with cache metadata
                        let truncated_text = cached.get_truncated(MAX_TOKENS_IN_RESPONSE);
                        let actual_cached = cached.total_tokens.min(MAX_CACHED_TOKENS);
                        let cache_footer = format!(
                            "\n\n--- OUTPUT TRUNCATED ---\n\
                             Original: {} tokens | Showing: {} tokens | Cached: {} tokens (0-{})\n\
                             Cache ID: '{}'\n\n\
                             ⚠️ CRITICAL: To explore cached content, you MUST:\n\
                             1. Use tool 'fetch_cached_output' with ALL THREE parameters:\n\
                                - ref_id='{}' (REQUIRED)\n\
                                - start_token=<number> (REQUIRED, 0-20999)\n\
                                - end_token=<number> (REQUIRED, 1-21000, MUST be numeric)\n\
                             2. Range size MUST NOT exceed 5000 tokens: (end_token - start_token ≤ 5000)\n\
                             3. DO NOT use 'end' or omit end_token - provide explicit numbers\n\n\
                             CORRECT exploration strategy (copy these exact calls):\n\
                               fetch_cached_output(ref_id='{}', start_token=0, end_token=5000)\n\
                               fetch_cached_output(ref_id='{}', start_token=5000, end_token=10000)\n\
                               fetch_cached_output(ref_id='{}', start_token=10000, end_token=15000)\n\
                               fetch_cached_output(ref_id='{}', start_token=15000, end_token=20000)\n\n\
                             If info not found in {} tokens, respond 'not found in available content'.",
                            cached.total_tokens,
                            MAX_TOKENS_IN_RESPONSE,
                            actual_cached,
                            actual_cached,
                            cached.ref_id,
                            cached.ref_id,
                            cached.ref_id,
                            cached.ref_id,
                            cached.ref_id,
                            cached.ref_id,
                            MAX_CACHED_TOKENS
                        );
                        
                        // Modify the content to truncated version
                        // Deserialize as JSON to manipulate content field
                        if let Ok(mut result_json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                            // Replace content with truncated text
                            result_json["content"] = serde_json::json!([{
                                "type": "text",
                                "text": format!("{}{}", truncated_text, cache_footer)
                            }]);
                            
                            // Deserialize back to CallToolResult
                            if let Ok(truncated_result) = serde_json::from_value::<CallToolResult>(result_json) {
                                return Ok(truncated_result);
                            }
                        }
                        
                        // Fallback: modify the existing result's content
                        call_result.content = vec![];
                        return Ok(call_result);
                    }
                }
            }
            return Ok(call_result);
        } else {
            return result;
        }
    }

    Err(format!("Tool {tool_name} not found"))
}

/// Fetches a specific range from a cached tool output
///
/// # Arguments
/// * `state` - Application state containing the tool output cache
/// * `ref_id` - Reference ID of the cached output
/// * `start_token` - Starting token position (0-based)
/// * `end_token` - Ending token position (exclusive)
///
/// # Returns
/// * `Result<String, String>` - The requested range of content if successful
#[tauri::command]
pub async fn fetch_cached_tool_output(
    state: State<'_, AppState>,
    ref_id: String,
    start_token: usize,
    end_token: usize,
) -> Result<String, String> {
    use crate::core::mcp::cache::get_cached_output;

    log::info!(
        "Fetching cached output: ref_id='{}', range={}-{}",
        ref_id,
        start_token,
        end_token
    );

    let cached = get_cached_output(&state.tool_output_cache, &ref_id).await?;
    let range_content = cached.get_range(start_token, end_token)?;

    Ok(range_content)
}

/// Cancels a running tool call by its cancellation token
///
/// # Arguments
/// * `state` - Application state containing cancellation tokens
/// * `cancellation_token` - Token identifying the tool call to cancel
///
/// # Returns
/// * `Result<(), String>` - Success if token found and cancelled, error otherwise
#[tauri::command]
pub async fn cancel_tool_call(
    state: State<'_, AppState>,
    cancellation_token: String,
) -> Result<(), String> {
    let mut cancellations = state.tool_call_cancellations.lock().await;
    
    if let Some(cancel_tx) = cancellations.remove(&cancellation_token) {
        // Send cancellation signal - ignore if receiver is already dropped
        let _ = cancel_tx.send(());
        println!("Tool call with token {cancellation_token} cancelled");
        Ok(())
    } else {
        Err(format!("Cancellation token {cancellation_token} not found"))
    }
}

fn parse_mcp_settings(value: Option<&Value>) -> McpSettings {
    value
        .and_then(|v| serde_json::from_value::<McpSettings>(v.clone()).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub async fn get_mcp_configs<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    let mut path = get_jan_data_folder_path(app.clone());
    path.push("mcp_config.json");

    // Create default empty config if file doesn't exist
    if !path.exists() {
        log::info!("mcp_config.json not found, creating default empty config");
        fs::write(&path, DEFAULT_MCP_CONFIG)
            .map_err(|e| format!("Failed to create default MCP config: {e}"))?;
    }

    let config_string = fs::read_to_string(&path).map_err(|e| e.to_string())?;

    let mut config_value: Value = if config_string.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&config_string).unwrap_or_else(|error| {
            log::error!("Failed to parse existing MCP config, regenerating defaults: {error}");
            json!({})
        })
    };

    if !config_value.is_object() {
        config_value = json!({});
    }

    let mut mutated = false;
    let config_object = config_value.as_object_mut().unwrap();

    let settings = parse_mcp_settings(config_object.get("mcpSettings"));
    if !config_object.contains_key("mcpSettings") {
        config_object.insert(
            "mcpSettings".to_string(),
            serde_json::to_value(&settings)
                .map_err(|e| format!("Failed to serialize MCP settings: {e}"))?,
        );
        mutated = true;
    }

    if !config_object.contains_key("mcpServers") {
        config_object.insert("mcpServers".to_string(), json!({}));
        mutated = true;
    }

    // Migration: Add Jan Browser MCP if not present
    let mcp_servers = config_object
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .ok_or("mcpServers is not an object")?;

    if !mcp_servers.contains_key("Jan Browser MCP") {
        log::info!("Migrating config: Adding 'Jan Browser MCP' server");
        mcp_servers.insert(
            "Jan Browser MCP".to_string(),
            json!({
                "command": "npx",
                "args": ["-y", "search-mcp-server@latest"],
                "env": {
                    "BRIDGE_HOST": "127.0.0.1",
                    "BRIDGE_PORT": "17389"
                },
                "active": false,
                "official": true
            }),
        );
        mutated = true;
    }

    // Persist any mutations back to disk
    if mutated {
        fs::write(
            &path,
            serde_json::to_string_pretty(&config_value)
                .map_err(|e| format!("Failed to serialize MCP config: {e}"))?,
        )
        .map_err(|e| format!("Failed to write MCP config: {e}"))?;
    }

    // Update in-memory state with latest settings
    {
        let state = app.state::<AppState>();
        let mut settings_guard = state.mcp_settings.lock().await;
        *settings_guard = settings.clone();
    }

    serde_json::to_string_pretty(&config_value)
        .map_err(|e| format!("Failed to serialize MCP config: {e}"))
}

#[tauri::command]
pub async fn save_mcp_configs<R: Runtime>(app: AppHandle<R>, configs: String) -> Result<(), String> {
    let mut path = get_jan_data_folder_path(app.clone());
    path.push("mcp_config.json");
    log::info!("save mcp configs, path: {path:?}");

    let mut config_value: Value = serde_json::from_str(&configs)
        .map_err(|e| format!("Invalid MCP config payload: {e}"))?;

    if !config_value.is_object() {
        return Err("MCP config must be a JSON object".to_string());
    }

    let config_object = config_value.as_object_mut().unwrap();
    let settings = parse_mcp_settings(config_object.get("mcpSettings"));

    if !config_object.contains_key("mcpSettings") {
        config_object.insert(
            "mcpSettings".to_string(),
            serde_json::to_value(&settings).expect("Failed to serialize MCP settings"),
        );
    }

    if !config_object.contains_key("mcpServers") {
        config_object.insert("mcpServers".to_string(), json!({}));
    }

    fs::write(
        &path,
        serde_json::to_string_pretty(&config_value)
            .map_err(|e| format!("Failed to serialize MCP config: {e}"))?,
    )
    .map_err(|e| e.to_string())?;

    {
        let state = app.state::<AppState>();
        let mut settings_guard = state.mcp_settings.lock().await;
        *settings_guard = settings;
    }

    Ok(())
}

// ============================================================================
// OAuth Commands for MCP Servers
// ============================================================================

use super::{
    models::{OAuthConfig, OAuthStatus},
    oauth::{save_oauth_tokens, start_oauth_flow},
};

/// Start OAuth authentication flow for an MCP server
#[tauri::command]
pub async fn start_mcp_oauth_flow<R: Runtime>(
    app: AppHandle<R>,
    server_name: String,
    oauth_config: OAuthConfig,
) -> Result<OAuthFlowResult, String> {
    log::info!("Starting OAuth flow for MCP server: {}", server_name);
    let auth_url = start_oauth_flow(app, server_name, oauth_config).await?;
    Ok(OAuthFlowResult { auth_url })
}

/// Get OAuth authentication status for an MCP server
#[tauri::command]
pub async fn get_mcp_oauth_status<R: Runtime>(
    app: AppHandle<R>,
    server_name: String,
) -> Result<OAuthStatus, String> {
    let state = app.state::<AppState>();
    let tokens = state.mcp_oauth_tokens.lock().await;

    match tokens.get(&server_name) {
        Some(token) => Ok(OAuthStatus {
            server_name,
            authenticated: !token.is_expired(),
            expires_at: Some(token.expires_at),
            scopes: token.scopes.clone(),
        }),
        None => Ok(OAuthStatus {
            server_name,
            authenticated: false,
            expires_at: None,
            scopes: vec![],
        }),
    }
}

/// Revoke OAuth token for an MCP server
#[tauri::command]
pub async fn revoke_mcp_oauth_token<R: Runtime>(
    app: AppHandle<R>,
    server_name: String,
) -> Result<(), String> {
    log::info!("Revoking OAuth token for MCP server: {}", server_name);

    let state = app.state::<AppState>();
    {
        let mut tokens = state.mcp_oauth_tokens.lock().await;
        tokens.remove(&server_name);
        save_oauth_tokens(&app, &tokens).await?;
    }

    // Emit event to frontend
    app.emit(
        "mcp_oauth_revoked",
        serde_json::json!({
            "server": server_name
        }),
    )
    .map_err(|e| format!("Failed to emit OAuth revoked event: {e}"))?;

    Ok(())
}

/// Get all OAuth statuses for MCP servers
#[tauri::command]
pub async fn get_all_mcp_oauth_statuses<R: Runtime>(
    app: AppHandle<R>,
) -> Result<HashMap<String, OAuthStatus>, String> {
    log::info!("[OAuth Debug] get_all_mcp_oauth_statuses called");
    let state = app.state::<AppState>();
    let tokens = state.mcp_oauth_tokens.lock().await;

    let mut statuses: HashMap<String, OAuthStatus> = tokens
        .iter()
        .map(|(server_name, token)| {
            log::info!("[OAuth Debug] Token store - server: {}, authenticated: {}", server_name, !token.is_expired());
            (
                server_name.clone(),
                OAuthStatus {
                    server_name: server_name.clone(),
                    authenticated: !token.is_expired(),
                    expires_at: Some(token.expires_at),
                    scopes: token.scopes.clone(),
                }
            )
        })
        .collect();
    
    drop(tokens);
    log::info!("[OAuth Debug] OAuth token store has {} entries", statuses.len());

    // Check for mcp-remote servers that store auth in ~/.mcp-auth
    let active_servers = state.mcp_active_servers.lock().await;
    log::info!("[OAuth Debug] Active servers count: {}", active_servers.len());
    
    // Check if ~/.mcp-auth exists and has content
    let mcp_remote_authenticated = if let Some(home_dir) = dirs::home_dir() {
        let mcp_auth_path = home_dir.join(".mcp-auth");
        log::info!("[OAuth Debug] Checking mcp-auth path: {:?}", mcp_auth_path);
        if mcp_auth_path.exists() {
            log::info!("[OAuth Debug] mcp-auth directory exists");
            if let Ok(entries) = std::fs::read_dir(&mcp_auth_path) {
                let has_files = entries.filter_map(|e| e.ok()).any(|_| true);
                log::info!("[OAuth Debug] mcp-auth has files: {}", has_files);
                has_files
            } else {
                log::warn!("[OAuth Debug] Could not read mcp-auth directory");
                false
            }
        } else {
            log::info!("[OAuth Debug] mcp-auth directory does not exist");
            false
        }
    } else {
        log::warn!("[OAuth Debug] Could not get home directory");
        false
    };
    
    // Check all active servers and override token store for mcp-remote servers
    for (server_name, config) in active_servers.iter() {
        log::info!("[OAuth Debug] Checking server: {}", server_name);
        
        // Check if this server uses mcp-remote
        if let Some(config_params) = extract_command_args(config) {
            let uses_mcp_remote = config_params.args.iter().any(|arg| {
                arg.as_str().map(|s| s.contains("mcp-remote")).unwrap_or(false)
            });
            
            log::info!("[OAuth Debug] Server {} uses mcp-remote: {}", server_name, uses_mcp_remote);
            
            if uses_mcp_remote {
                // For mcp-remote servers, always use ~/.mcp-auth check (override token store)
                log::info!("[OAuth Debug] Overriding token store status for mcp-remote server: {}", server_name);
                statuses.insert(
                    server_name.clone(),
                    OAuthStatus {
                        server_name: server_name.clone(),
                        authenticated: mcp_remote_authenticated,
                        expires_at: None,
                        scopes: Vec::new(),
                    }
                );
                log::info!("[OAuth Debug] Set mcp-remote server {} authenticated={}", server_name, mcp_remote_authenticated);
            }
        } else {
            log::info!("[OAuth Debug] Could not extract command args for server: {}", server_name);
        }
    }

    log::info!("[OAuth Debug] Returning {} OAuth statuses", statuses.len());
    for (name, status) in statuses.iter() {
        log::info!("[OAuth Debug] Status - {}: authenticated={}", name, status.authenticated);
    }

    Ok(statuses)
}

/// Clear MCP remote auth folder (~/.mcp-auth)
/// This is useful for MCP servers that use mcp-remote for OAuth
#[tauri::command]
pub async fn clear_mcp_remote_auth<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| "Failed to get home directory".to_string())?;
    
    let mcp_auth_path = home_dir.join(".mcp-auth");
    
    if !mcp_auth_path.exists() {
        log::info!("MCP auth folder does not exist: {:?}", mcp_auth_path);
        return Ok(());
    }
    
    log::info!("Clearing MCP remote auth folder: {:?}", mcp_auth_path);
    
    // Remove all contents of the directory
    fs::remove_dir_all(&mcp_auth_path)
        .map_err(|e| format!("Failed to remove MCP auth folder: {}", e))?;
    
    // Recreate the empty directory
    fs::create_dir(&mcp_auth_path)
        .map_err(|e| format!("Failed to recreate MCP auth folder: {}", e))?;
    
    // Reset all OAuth URL opened flags so servers can prompt again
    let state = app.state::<AppState>();
    {
        let mut opened = state.mcp_oauth_url_opened.lock().await;
        opened.clear();
    }
    
    log::info!("Successfully cleared MCP remote auth folder and reset OAuth flags");
    Ok(())
}

/// Clears all cached tool outputs
///
/// # Arguments
/// * `state` - Application state containing the tool output cache
///
/// # Returns
/// * `Result<(), String>` - Success if cache was cleared
#[tauri::command]
pub async fn clear_tool_output_cache(state: State<'_, AppState>) -> Result<(), String> {
    log::info!("Clearing tool output cache");
    state.tool_output_cache.clear().await;
    Ok(())
}

/// Gets statistics about the tool output cache
///
/// # Arguments
/// * `state` - Application state containing the tool output cache
///
/// # Returns
/// * `Result<CacheStats, String>` - Cache statistics if successful
#[tauri::command]
pub async fn get_cache_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    use crate::core::mcp::cache::CacheStats;
    
    let stats = state.tool_output_cache.stats().await;
    Ok(serde_json::json!({
        "totalEntries": stats.total_entries,
        "totalBytes": stats.total_bytes,
        "totalTokens": stats.total_tokens
    }))
}

