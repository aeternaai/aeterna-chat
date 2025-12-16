#!/bin/bash
# Test OAuth Flow via curl (simulating the callback)

# This script shows what happens during OAuth flow

echo "=== OAuth Flow Test ==="
echo ""

# Step 1: Normally, start_mcp_oauth_flow would be called from UI
# This returns an auth_url like:
# https://auth.atlassian.com/authorize?
#   client_id=YOUR_CLIENT_ID
#   &redirect_uri=http://localhost:17390/callback
#   &response_type=code
#   &scope=read:jira-work...
#   &state=RANDOM_STATE

echo "Step 1: User clicks 'Authenticate' button in UI"
echo "Backend generates OAuth URL and starts callback server on port 17390"
echo ""

# Step 2: Browser opens to auth_url
echo "Step 2: Browser opens to Atlassian OAuth page"
echo "User logs in and grants permissions"
echo ""

# Step 3: OAuth provider redirects to callback
echo "Step 3: Atlassian redirects to http://localhost:17390/callback?code=AUTH_CODE&state=STATE"
echo ""

# Step 4: Simulate the callback (this is what Atlassian would do)
# NOTE: You need a REAL authorization code from Atlassian for this to work
AUTH_CODE="YOUR_AUTH_CODE_HERE"
STATE="test-state-12345"

echo "Simulating callback with code=$AUTH_CODE"
curl -v "http://localhost:17390/callback?code=$AUTH_CODE&state=$STATE" 2>&1 | head -20

echo ""
echo "Step 4: Backend exchanges code for access token"
echo "Token stored in: ~/Library/Application Support/jan/mcp_oauth_tokens.json"
echo ""

# Step 5: Check if token was stored
if [ -f "$HOME/Library/Application Support/jan/mcp_oauth_tokens.json" ]; then
    echo "Step 5: Token file exists!"
    echo "Contents:"
    cat "$HOME/Library/Application Support/jan/mcp_oauth_tokens.json" | jq '.'
else
    echo "Step 5: Token file not found yet"
fi

echo ""
echo "=== Testing Tips ==="
echo "1. Get real OAuth credentials from Atlassian:"
echo "   - Go to https://developer.atlassian.com/"
echo "   - Create an OAuth 2.0 integration"
echo "   - Set redirect URI to: http://localhost:17390/callback"
echo "   - Copy client_id and client_secret"
echo ""
echo "2. Update jira-rovo config in Jan with your client_id"
echo ""
echo "3. Start Jan and click the blue key icon on JIRA ROVO server"
echo ""
echo "4. Watch the logs:"
echo "   tail -f ~/Library/Logs/Jan/app.log | grep -i oauth"
