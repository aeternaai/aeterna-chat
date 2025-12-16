use crate::core::{
    app::commands::get_jan_data_folder_path,
    mcp::models::{OAuthConfig, OAuthToken},
};
use rand::Rng;
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_http::reqwest;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::Mutex,
};

/// OAuth token response from authorization server
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: String,
    expires_in: u64,
    scope: Option<String>,
}

/// In-memory storage for OAuth tokens (persisted to disk)
pub type OAuthTokenStore = Arc<Mutex<HashMap<String, OAuthToken>>>;

/// Load OAuth tokens from disk
pub async fn load_oauth_tokens<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<HashMap<String, OAuthToken>, String> {
    let data_folder = get_jan_data_folder_path(app.clone());
    let tokens_path = data_folder.join("mcp_oauth_tokens.json");

    if !tokens_path.exists() {
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(&tokens_path)
        .map_err(|e| format!("Failed to read OAuth tokens: {e}"))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse OAuth tokens: {e}"))
}

/// Save OAuth tokens to disk
pub async fn save_oauth_tokens<R: Runtime>(
    app: &AppHandle<R>,
    tokens: &HashMap<String, OAuthToken>,
) -> Result<(), String> {
    let data_folder = get_jan_data_folder_path(app.clone());
    let tokens_path = data_folder.join("mcp_oauth_tokens.json");

    let content = serde_json::to_string_pretty(tokens)
        .map_err(|e| format!("Failed to serialize OAuth tokens: {e}"))?;

    std::fs::write(&tokens_path, content)
        .map_err(|e| format!("Failed to write OAuth tokens: {e}"))
}

/// Generate a random state parameter for CSRF protection
fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..62);
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"[idx] as char
        })
        .collect()
}

/// Start OAuth flow for an MCP server
pub async fn start_oauth_flow<R: Runtime>(
    app: AppHandle<R>,
    server_name: String,
    oauth_config: OAuthConfig,
) -> Result<String, String> {
    // Generate CSRF state
    let state = generate_state();

    // Determine redirect URI
    let redirect_uri = oauth_config
        .redirect_uri
        .clone()
        .unwrap_or_else(|| "http://localhost:17390/oauth/callback".to_string());

    // Build authorization URL
    let mut auth_url = reqwest::Url::parse(&oauth_config.auth_url)
        .map_err(|e| format!("Invalid auth URL: {e}"))?;

    auth_url
        .query_pairs_mut()
        .append_pair("client_id", &oauth_config.client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("state", &state)
        .append_pair("scope", &oauth_config.scopes.join(" "));

    // Store pending OAuth flow in app state
    let app_state = app.state::<crate::core::state::AppState>();
    {
        let mut pending = app_state.mcp_oauth_pending.lock().await;
        pending.insert(
            state.clone(),
            PendingOAuthFlow {
                server_name: server_name.clone(),
                oauth_config: oauth_config.clone(),
                redirect_uri: redirect_uri.clone(),
            },
        );
    }

    // Start local callback server if not already running
    ensure_oauth_callback_server_running(app.clone()).await?;

    // Log the authorization URL for debugging
    log::info!("Generated OAuth authorization URL: {}", auth_url.to_string());

    // Return the authorization URL for the frontend to open
    Ok(auth_url.to_string())
}

/// Handle OAuth callback with authorization code
pub async fn handle_oauth_callback<R: Runtime>(
    app: AppHandle<R>,
    code: String,
    state: String,
) -> Result<(), String> {
    let app_state = app.state::<crate::core::state::AppState>();

    // Retrieve pending OAuth flow
    let pending_flow = {
        let mut pending = app_state.mcp_oauth_pending.lock().await;
        pending
            .remove(&state)
            .ok_or_else(|| "Invalid or expired OAuth state".to_string())?
    };

    // Exchange authorization code for tokens
    let token_response = exchange_code_for_token(
        &pending_flow.oauth_config,
        &code,
        &pending_flow.redirect_uri,
    )
    .await?;

    // Calculate expiration timestamp
    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + token_response.expires_in as i64;

    // Parse scopes
    let scopes = token_response
        .scope
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_else(|| pending_flow.oauth_config.scopes.clone());

    // Create OAuth token
    let oauth_token = OAuthToken {
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
        token_type: token_response.token_type,
        expires_at,
        scopes,
    };

    // Store token
    {
        let mut tokens = app_state.mcp_oauth_tokens.lock().await;
        tokens.insert(pending_flow.server_name.clone(), oauth_token);
        save_oauth_tokens(&app, &tokens).await?;
    }

    // Emit event to frontend
    app.emit(
        "mcp_oauth_complete",
        serde_json::json!({
            "server": pending_flow.server_name,
            "success": true
        }),
    )
    .map_err(|e| format!("Failed to emit OAuth complete event: {e}"))?;

    log::info!("OAuth flow completed for server: {}", pending_flow.server_name);

    Ok(())
}

/// Exchange authorization code for access token
async fn exchange_code_for_token(
    oauth_config: &OAuthConfig,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, String> {
    let client = reqwest::Client::new();

    let params = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
    ];

    // Build request with HTTP Basic Authentication if client_secret is available
    let mut request = client
        .post(&oauth_config.token_url)
        .form(&params);

    // Atlassian OAuth requires HTTP Basic Auth with client_id:client_secret
    if let Some(ref secret) = oauth_config.client_secret {
        request = request.basic_auth(&oauth_config.client_id, Some(secret));
    } else {
        // Fallback: send client_id as form parameter for public clients
        request = request.form(&[("client_id", &oauth_config.client_id)]);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Token exchange request failed: {e}"))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Token exchange failed: {error_text}"));
    }

    response
        .json::<TokenResponse>()
        .await
        .map_err(|e| format!("Failed to parse token response: {e}"))
}

/// Refresh an OAuth token
pub async fn refresh_oauth_token<R: Runtime>(
    app: AppHandle<R>,
    server_name: String,
    oauth_config: OAuthConfig,
) -> Result<(), String> {
    let app_state = app.state::<crate::core::state::AppState>();

    // Get current token
    let refresh_token = {
        let tokens = app_state.mcp_oauth_tokens.lock().await;
        tokens
            .get(&server_name)
            .and_then(|t| t.refresh_token.clone())
            .ok_or_else(|| "No refresh token available".to_string())?
    };

    // Request new token
    let client = reqwest::Client::new();
    let params = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", &refresh_token),
    ];

    // Build request with HTTP Basic Authentication if client_secret is available
    let mut request = client
        .post(&oauth_config.token_url)
        .form(&params);

    // Atlassian OAuth requires HTTP Basic Auth with client_id:client_secret
    if let Some(ref secret) = oauth_config.client_secret {
        request = request.basic_auth(&oauth_config.client_id, Some(secret));
    } else {
        // Fallback: send client_id as form parameter for public clients
        request = request.form(&[("client_id", &oauth_config.client_id)]);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Token refresh request failed: {e}"))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Token refresh failed: {error_text}"));
    }

    let token_response: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse refresh token response: {e}"))?;

    // Calculate new expiration
    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + token_response.expires_in as i64;

    let scopes = token_response
        .scope
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_else(|| oauth_config.scopes.clone());

    // Update stored token
    let oauth_token = OAuthToken {
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token.or(Some(refresh_token)),
        token_type: token_response.token_type,
        expires_at,
        scopes,
    };

    {
        let mut tokens = app_state.mcp_oauth_tokens.lock().await;
        tokens.insert(server_name.clone(), oauth_token);
        save_oauth_tokens(&app, &tokens).await?;
    }

    log::info!("OAuth token refreshed for server: {}", server_name);

    Ok(())
}

/// Pending OAuth flow data
#[derive(Debug, Clone)]
pub struct PendingOAuthFlow {
    pub server_name: String,
    pub oauth_config: OAuthConfig,
    pub redirect_uri: String,
}

/// Ensure OAuth callback server is running
async fn ensure_oauth_callback_server_running<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    let app_state = app.state::<crate::core::state::AppState>();

    // Check if already running
    {
        let server = app_state.mcp_oauth_server.lock().await;
        if server.is_some() {
            return Ok(());
        }
    }

    // Start callback server
    let listener = TcpListener::bind("127.0.0.1:17390")
        .await
        .map_err(|e| format!("Failed to bind OAuth callback server: {e}"))?;

    log::info!("OAuth callback server started on http://localhost:17390");

    let app_clone = app.clone();
    let handle = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let app = app_clone.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_callback_request(app, stream).await {
                            log::error!("Failed to handle OAuth callback: {e}");
                        }
                    });
                }
                Err(e) => {
                    log::error!("Failed to accept OAuth callback connection: {e}");
                }
            }
        }
    });

    // Store handle
    {
        let mut server = app_state.mcp_oauth_server.lock().await;
        *server = Some(handle);
    }

    Ok(())
}

/// Handle individual OAuth callback HTTP request
async fn handle_callback_request<R: Runtime>(
    app: AppHandle<R>,
    stream: tokio::net::TcpStream,
) -> Result<(), String> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read HTTP request line
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .await
        .map_err(|e| format!("Failed to read request: {e}"))?;

    // Parse URL
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid HTTP request".to_string());
    }

    let path = parts[1];

    // Check if this is the OAuth callback
    if !path.starts_with("/oauth/callback") {
        send_response(&mut writer, 404, "Not Found").await?;
        return Ok(());
    }

    // Parse query parameters
    let query_start = path.find('?').unwrap_or(path.len());
    let query = &path[query_start + 1..];
    let params: HashMap<String, String> = query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            Some((
                parts.next()?.to_string(),
                urlencoding::decode(parts.next()?).ok()?.to_string(),
            ))
        })
        .collect();

    // Extract code and state
    let code = params
        .get("code")
        .ok_or_else(|| "Missing authorization code".to_string())?;
    let state = params
        .get("state")
        .ok_or_else(|| "Missing state parameter".to_string())?;

    // Handle callback in background
    let app_clone = app.clone();
    let code_clone = code.clone();
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = handle_oauth_callback(app_clone, code_clone, state_clone).await {
            log::error!("OAuth callback handling failed: {e}");
        }
    });

    // Send success response
    send_response(
        &mut writer,
        200,
        r#"
        <!DOCTYPE html>
        <html>
        <head><title>Authentication Successful</title></head>
        <body style="font-family: sans-serif; text-align: center; padding-top: 100px;">
            <h1>✓ Authentication Successful</h1>
            <p>You can close this window and return to Jan.</p>
            <script>setTimeout(() => window.close(), 2000);</script>
        </body>
        </html>
        "#,
    )
    .await?;

    Ok(())
}

/// Send HTTP response
async fn send_response(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    status: u16,
    body: &str,
) -> Result<(), String> {
    let status_text = match status {
        200 => "OK",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };

    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        status,
        status_text,
        body.len(),
        body
    );

    writer
        .write_all(response.as_bytes())
        .await
        .map_err(|e| format!("Failed to write response: {e}"))?;

    Ok(())
}

/// Get OAuth token for a server (refreshing if needed)
pub async fn get_oauth_token<R: Runtime>(
    app: AppHandle<R>,
    server_name: &str,
    oauth_config: &OAuthConfig,
) -> Result<Option<String>, String> {
    let app_state = app.state::<crate::core::state::AppState>();

    let token = {
        let tokens = app_state.mcp_oauth_tokens.lock().await;
        tokens.get(server_name).cloned()
    };

    match token {
        Some(mut token) => {
            // Check if token needs refresh
            if token.is_expired() {
                if token.refresh_token.is_some() {
                    log::info!("Refreshing expired OAuth token for {}", server_name);
                    let app_clone = app.clone(); // Clone app handle to avoid borrow issues
                    refresh_oauth_token(app_clone, server_name.to_string(), oauth_config.clone())
                        .await?;

                    // Get refreshed token
                    let tokens = app_state.mcp_oauth_tokens.lock().await;
                    token = tokens
                        .get(server_name)
                        .cloned()
                        .ok_or_else(|| "Token refresh failed".to_string())?;
                } else {
                    return Err(format!(
                        "OAuth token expired and no refresh token available for {}",
                        server_name
                    ));
                }
            }

            let header_value = format!("{} {}", token.token_type, token.access_token);
            log::debug!("Returning OAuth token header value: {}", header_value);
            Ok(Some(header_value))
        }
        None => Ok(None),
    }
}
