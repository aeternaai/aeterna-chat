// Test OAuth Flow from Browser Console
// Open DevTools in Jan app (Cmd+Option+I on macOS)
// Paste this into the console:

// Start OAuth flow
const { invoke } = window.__TAURI__.core;

// Step 1: Start OAuth flow for JIRA ROVO
invoke('start_mcp_oauth_flow', { serverName: 'jira-rovo' })
  .then(result => {
    console.log('OAuth URL:', result.auth_url);
    // Manually open this URL in your browser
    window.open(result.auth_url, '_blank');
  })
  .catch(err => console.error('OAuth flow failed:', err));

// Step 2: Check OAuth status
invoke('get_mcp_oauth_status', { serverName: 'jira-rovo' })
  .then(status => {
    console.log('OAuth Status:', status);
    // Shows: { authenticated: true/false, server_name: "jira-rovo", expires_at?: number }
  });

// Step 3: Get all OAuth statuses
invoke('get_all_mcp_oauth_statuses')
  .then(statuses => {
    console.log('All OAuth Statuses:', statuses);
  });

// Step 4: Listen for OAuth completion event
const { event } = window.__TAURI__;
event.listen('mcp_oauth_complete', (event) => {
  console.log('OAuth completed!', event.payload);
  // Payload: { server_name: string, authenticated: boolean, expires_at?: number }
});

// Step 5: Revoke OAuth token (if needed)
invoke('revoke_mcp_oauth_token', { serverName: 'jira-rovo' })
  .then(() => console.log('Token revoked'))
  .catch(err => console.error('Revoke failed:', err));
