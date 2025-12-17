#!/bin/bash

# Script to add Atlassian OAuth client secret to MCP config

echo "════════════════════════════════════════════════════════════════"
echo "  Add Atlassian OAuth Client Secret to Jan Configuration"
echo "════════════════════════════════════════════════════════════════"
echo

# Prompt for client secret
read -sp "Enter your Atlassian OAuth Client Secret: " CLIENT_SECRET
echo
echo

if [ -z "$CLIENT_SECRET" ]; then
    echo "❌ Error: Client secret cannot be empty"
    exit 1
fi

CONFIG_FILE="$HOME/Library/Application Support/Jan/data/mcp_config.json"

if [ ! -f "$CONFIG_FILE" ]; then
    echo "❌ Error: Config file not found at: $CONFIG_FILE"
    exit 1
fi

echo "📝 Adding client secret to jira-rovo OAuth configuration..."

# Create backup
cp "$CONFIG_FILE" "$CONFIG_FILE.backup_$(date +%Y%m%d_%H%M%S)"

# Update config with clientSecret (camelCase to match other fields)
jq ".mcpServers[\"jira-rovo\"].oauth.clientSecret = \"$CLIENT_SECRET\"" "$CONFIG_FILE" > /tmp/mcp_config_updated.json

if [ $? -eq 0 ]; then
    mv /tmp/mcp_config_updated.json "$CONFIG_FILE"
    echo "✅ Client secret added successfully!"
    echo
    echo "📋 Updated OAuth configuration:"
    jq '.mcpServers["jira-rovo"].oauth | {clientId, clientSecret: "***REDACTED***", authUrl, tokenUrl, redirectUri, scopes}' "$CONFIG_FILE"
    echo
    echo "════════════════════════════════════════════════════════════════"
    echo "  Next Steps:"
    echo "════════════════════════════════════════════════════════════════"
    echo "1. Restart Jan (Ctrl+C in the terminal running 'make dev', then run 'make dev' again)"
    echo "2. Go to Settings → MCP Servers"
    echo "3. Click the blue key icon (🔑) on jira-rovo"
    echo "4. Browser should open automatically and authentication should work!"
    echo
else
    echo "❌ Error updating config file"
    exit 1
fi
