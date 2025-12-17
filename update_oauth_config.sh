#!/bin/bash
# Update JIRA ROVO OAuth Client ID in Jan's Configuration

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Update JIRA ROVO OAuth Configuration"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Detect OS and set config path
if [[ "$OSTYPE" == "darwin"* ]]; then
    CONFIG_DIR="$HOME/Library/Application Support/jan"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    CONFIG_DIR="$HOME/.config/jan"
else
    echo "❌ Unsupported OS: $OSTYPE"
    exit 1
fi

CONFIG_FILE="$CONFIG_DIR/mcp_config.json"

# Check if config file exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "⚠️  Config file not found: $CONFIG_FILE"
    echo ""
    echo "This is normal if Jan hasn't been run yet."
    echo "Options:"
    echo "  1. Start Jan first with: make dev"
    echo "  2. Or create the config manually"
    echo ""
    read -p "Create default config now? (y/n): " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        mkdir -p "$CONFIG_DIR"
        cat > "$CONFIG_FILE" << 'EOF'
{
  "mcpServers": {},
  "mcpSettings": {
    "toolCallTimeoutSeconds": 30,
    "baseRestartDelayMs": 1000,
    "maxRestartDelayMs": 30000,
    "backoffMultiplier": 2,
    "proactiveMode": false
  }
}
EOF
        echo "✅ Created default config"
    else
        echo "Exiting..."
        exit 0
    fi
fi

echo "📁 Config file: $CONFIG_FILE"
echo ""

# Prompt for client ID
echo "Enter your Atlassian OAuth Client ID:"
echo "(Get it from: https://developer.atlassian.com/console/myapps/)"
echo ""
read -p "Client ID: " CLIENT_ID

if [ -z "$CLIENT_ID" ]; then
    echo "❌ Client ID cannot be empty"
    exit 1
fi

echo ""
read -p "Do you also want to add Client Secret as environment variable? (y/n): " -n 1 -r
echo ""
ADD_SECRET=false
if [[ $REPLY =~ ^[Yy]$ ]]; then
    ADD_SECRET=true
    echo "Enter your Atlassian OAuth Client Secret:"
    read -s -p "Client Secret: " CLIENT_SECRET
    echo ""
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Updating Configuration..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Create backup
BACKUP_FILE="$CONFIG_FILE.backup.$(date +%Y%m%d_%H%M%S)"
cp "$CONFIG_FILE" "$BACKUP_FILE"
echo "✅ Backup created: $BACKUP_FILE"

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    echo "⚠️  jq is not installed. Installing via Homebrew..."
    if command -v brew &> /dev/null; then
        brew install jq
    else
        echo "❌ Homebrew not found. Please install jq manually:"
        echo "   brew install jq"
        exit 1
    fi
fi

# Update the config using jq
TMP_FILE=$(mktemp)

# Check if jira-rovo exists
if jq -e '.mcpServers["jira-rovo"]' "$CONFIG_FILE" > /dev/null 2>&1; then
    echo "📝 Updating existing jira-rovo configuration..."
    jq ".mcpServers[\"jira-rovo\"].oauth.client_id = \"$CLIENT_ID\"" "$CONFIG_FILE" > "$TMP_FILE"
else
    echo "📝 Creating new jira-rovo configuration..."
    jq ".mcpServers[\"jira-rovo\"] = {
      \"command\": \"npx\",
      \"args\": [\"-y\", \"mcp-remote\", \"https://mcp.atlassian.com/v1/sse\", \"--transport\", \"sse-only\"],
      \"type\": \"sse\",
      \"url\": \"https://mcp.atlassian.com/v1/sse\",
      \"env\": {},
      \"active\": true,
      \"official\": false,
      \"oauth\": {
        \"client_id\": \"$CLIENT_ID\",
        \"auth_url\": \"https://auth.atlassian.com/authorize\",
        \"token_url\": \"https://auth.atlassian.com/oauth/token\",
        \"scopes\": [
          \"read:jira-work\",
          \"read:confluence-content.summary\",
          \"read:jira-user\",
          \"offline_access\"
        ]
      }
    }" "$CONFIG_FILE" > "$TMP_FILE"
fi

mv "$TMP_FILE" "$CONFIG_FILE"
echo "✅ Configuration updated successfully"

# Add client secret to environment if requested
if [ "$ADD_SECRET" = true ]; then
    SHELL_RC=""
    if [ -n "$ZSH_VERSION" ]; then
        SHELL_RC="$HOME/.zshrc"
    elif [ -n "$BASH_VERSION" ]; then
        SHELL_RC="$HOME/.bashrc"
    fi
    
    if [ -n "$SHELL_RC" ]; then
        echo "" >> "$SHELL_RC"
        echo "# Atlassian OAuth Client Secret for Jan MCP" >> "$SHELL_RC"
        echo "export JIRA_ROVO_CLIENT_SECRET=\"$CLIENT_SECRET\"" >> "$SHELL_RC"
        echo "✅ Added JIRA_ROVO_CLIENT_SECRET to $SHELL_RC"
        echo "   Run: source $SHELL_RC"
    else
        echo "⚠️  Could not detect shell. Please add manually:"
        echo "   export JIRA_ROVO_CLIENT_SECRET=\"$CLIENT_SECRET\""
    fi
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ✅ Configuration Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📋 Summary:"
echo "   • Client ID: $CLIENT_ID"
echo "   • Config file: $CONFIG_FILE"
echo "   • Backup: $BACKUP_FILE"
echo ""
echo "🔍 Verify configuration:"
echo "   cat '$CONFIG_FILE' | jq '.mcpServers[\"jira-rovo\"].oauth'"
echo ""
echo "🚀 Next steps:"
echo "   1. Make sure redirect URI is set in Atlassian OAuth app:"
echo "      http://localhost:17390/callback"
echo ""
if [ "$ADD_SECRET" = true ]; then
    echo "   2. Reload your shell environment:"
    if [ -n "$SHELL_RC" ]; then
        echo "      source $SHELL_RC"
    fi
    echo ""
    echo "   3. Start Jan:"
else
    echo "   2. Set your client secret (choose one method):"
    echo "      a) Environment variable (recommended):"
    echo "         export JIRA_ROVO_CLIENT_SECRET=\"your-secret\""
    echo "      b) Or add to shell profile:"
    echo "         echo 'export JIRA_ROVO_CLIENT_SECRET=\"your-secret\"' >> ~/.zshrc"
    echo ""
    echo "   3. Start Jan:"
fi
echo "      make dev"
echo ""
echo "   4. Go to Settings → MCP Servers"
echo "   5. Click the blue key icon (🔑) on jira-rovo"
echo "   6. Complete OAuth in your browser"
echo ""
echo "📚 For detailed help, see:"
echo "   • OAUTH_TESTING_GUIDE.md"
echo "   • HOW_TO_ADD_OAUTH_CLIENT_ID.md"
echo ""
