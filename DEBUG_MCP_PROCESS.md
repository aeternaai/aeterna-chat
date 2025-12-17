# Debug MCP Process Behavior

## Test: Monitor mcp-remote process lifecycle

### Setup
1. Ensure jira-rovo is **disabled** in Jan Settings → MCP Servers
2. Open Terminal
3. Run this monitoring script:

```bash
while true; do
  ps aux | grep "mcp-remote" | grep -v grep | grep atlassian
  sleep 1
done
```

### Steps
1. Start the monitoring script above in terminal
2. In Jan, activate jira-rovo
3. Watch the terminal to see if mcp-remote process:
   - Starts
   - Stays running
   - Exits/crashes
   - Gets restarted multiple times

### Expected Observations

**Scenario A: Process Crashes**
```
# Process appears
node ... /mcp-remote https://mcp.atlassian.com/v1/sse
# Process disappears after 2-3 seconds
# Process reappears (Jan restarts it)
# Repeat
```
→ This means mcp-remote itself is crashing

**Scenario B: Process Stays Running**
```
# Process appears
node ... /mcp-remote https://mcp.atlassian.com/v1/sse
# Process stays visible even during "Transport closed" errors
```
→ This means the SSE connection is closing but mcp-remote is still running

### What to Log
1. Timestamp when process first appears
2. Whether process stays running or exits
3. How many times it restarts
4. Any pattern in timing (does it crash after exactly X seconds?)

## Alternative Test: Check mcp-remote stdout

Since Jan only captures stderr on startup failure, we're missing mcp-remote's normal output.

Try this wrapper script:

```bash
#!/bin/bash
# Save as /tmp/mcp-remote-wrapper.sh
# chmod +x /tmp/mcp-remote-wrapper.sh

exec npx -y mcp-remote "$@" 2>&1 | tee -a /tmp/mcp-remote.log
```

Then change jira-rovo config to use this wrapper:
```json
{
  "command": "/tmp/mcp-remote-wrapper.sh",
  "args": ["https://mcp.atlassian.com/v1/sse"],
  ...
}
```

This will log ALL output (stdout + stderr) to `/tmp/mcp-remote.log`

## Questions to Answer
1. Does mcp-remote process exit, or does it stay running?
2. What does mcp-remote output when "Transport closed" happens?
3. Is there an Atlassian-side timeout or rate limit?
4. Does the OAuth token actually work, or does it get rejected?
