# JIRA MCP Server - Implementation Plan

**Status**: Planning Phase  
**Timeline**: 4-6 weeks  
**Complexity**: Medium-High  
**Last Updated**: December 4, 2025

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [JIRA MCP Server Specification](#jira-mcp-server-specification)
4. [Implementation Phases](#implementation-phases)
5. [Testing & Quality Assurance](#testing--quality-assurance)
6. [Scalability & Reusability](#scalability--reusability)
7. [Publishing & Distribution](#publishing--distribution)
8. [Risk Mitigation](#risk-mitigation)
9. [Success Metrics](#success-metrics)

---

## Executive Summary

### Goal
Enable Jan users to interact with JIRA projects, issues, and workflows through natural language conversation.

### Scope
Full JIRA integration supporting:
- Issue management (create, read, update, delete, search)
- Project navigation and workflow operations
- Team collaboration features
- Sprint/Agile board management

### Timeline Estimate
- **Phase 1**: MCP Server Development (Weeks 1-2)
- **Phase 2**: Jan Integration (Weeks 3-4)
- **Phase 3**: Documentation & Testing (Week 5)
- **Phase 4**: Publishing & Release (Week 6)

### Complexity Assessment
**Medium-High** due to:
- External API integration requirements
- Multiple authentication methods
- Complex JIRA API with ~50 endpoints
- Comprehensive tool definitions needed
- Security considerations for credential handling

---

## Architecture Overview

### Jan's MCP Architecture (Three-Layer System)

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend Layer (TypeScript)               │
│  - React UI: MCP server configuration forms                 │
│  - Tool selection UI and permission dialogs                 │
│  - Tool call visualization                                   │
│  - State management (Zustand)                                │
└─────────────────────────┬───────────────────────────────────┘
                          │ 
                          │ Tauri IPC Commands
                          │ (invoke/emit)
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                   Backend Layer (Rust)                       │
│  - MCP server lifecycle (start/stop/restart/monitor)        │
│  - Transport layer (stdio, HTTP, SSE)                       │
│  - Tool discovery and routing                               │
│  - Error handling with exponential backoff                  │
│  - Configuration persistence (mcp_config.json)              │
│  - Lock file management (for port conflicts)                │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          │ Child Process / Network
                          │
┌─────────────────────────▼───────────────────────────────────┐
│              External MCP Server (Node.js/Python)            │
│  - JIRA API client (REST API v3)                            │
│  - MCP protocol implementation                               │
│  - Tool definitions and handlers                            │
│  - Authentication (API Token/PAT/OAuth)                      │
│  - Rate limiting and error handling                          │
└─────────────────────────────────────────────────────────────┘
```

### Key Components in Jan Repository

#### Backend (Rust)
- **`src-tauri/src/core/mcp/commands.rs`**
  - Tauri commands: `activate_mcp_server`, `deactivate_mcp_server`, `get_tools`, `call_tool`
  - Server lifecycle management
  - Tool call routing with timeout support
  
- **`src-tauri/src/core/mcp/helpers.rs`**
  - Server startup with restart monitoring
  - Transport initialization (stdio/HTTP/SSE)
  - Exponential backoff retry logic
  - Health monitoring
  
- **`src-tauri/src/core/mcp/models.rs`**
  - `McpServerConfig` - Configuration parameters
  - `McpSettings` - Runtime settings (timeouts, backoff)
  - `ToolWithServer` - Tool metadata
  
- **`src-tauri/src/core/mcp/constants.rs`**
  - `DEFAULT_MCP_CONFIG` - Default server configurations
  - Timeout and retry constants

#### Frontend (TypeScript)
- **`web-app/src/hooks/useMCPServers.ts`**
  - Zustand store for MCP server state
  - Configuration management
  
- **`web-app/src/services/mcp/tauri.ts`**
  - Tauri-specific MCP service implementation
  - Wraps Tauri IPC calls
  
- **`web-app/src/containers/ChatInput.tsx`**
  - Tool UI integration
  - Tool selection dropdown
  
- **MCP Settings UI Components** (to be created)
  - Configuration forms
  - Server management

#### Configuration Files
- **`mcp_config.json`** (in Jan data folder)
  - User's MCP server configurations
  - Structure:
    ```json
    {
      "mcpServers": {
        "server-name": {
          "command": "npx",
          "args": ["-y", "package-name"],
          "env": { "KEY": "value" },
          "active": true/false,
          "official": true/false
        }
      },
      "mcpSettings": {
        "toolCallTimeoutSeconds": 30,
        "baseRestartDelayMs": 1000,
        "maxRestartDelayMs": 30000,
        "backoffMultiplier": 2.0,
        "proactiveMode": false
      }
    }
    ```

---

## JIRA MCP Server Specification

### Core Features (30+ Tools)

#### Issue Management Tools (12 tools)
| Tool Name | Purpose | Required Params |
|-----------|---------|-----------------|
| `jira_create_issue` | Create new issue | project, summary, issueType |
| `jira_get_issue` | Get issue details | issueKey |
| `jira_update_issue` | Update issue fields | issueKey, fields |
| `jira_delete_issue` | Delete issue | issueKey |
| `jira_search_issues` | JQL-based search | jql |
| `jira_assign_issue` | Assign to user | issueKey, assignee |
| `jira_transition_issue` | Move through workflow | issueKey, transitionId |
| `jira_link_issues` | Link two issues | inwardIssue, outwardIssue, linkType |
| `jira_add_comment` | Add comment | issueKey, comment |
| `jira_get_comments` | List comments | issueKey |
| `jira_add_attachment` | Upload file | issueKey, file |
| `jira_get_attachments` | List attachments | issueKey |

#### Project & Workflow Tools (8 tools)
| Tool Name | Purpose | Required Params |
|-----------|---------|-----------------|
| `jira_list_projects` | List all projects | - |
| `jira_get_project` | Get project details | projectKey |
| `jira_list_issue_types` | List issue types | projectKey |
| `jira_list_priorities` | List priorities | - |
| `jira_list_statuses` | List workflow statuses | - |
| `jira_get_transitions` | Get available transitions | issueKey |
| `jira_list_components` | List project components | projectKey |
| `jira_list_versions` | List versions/releases | projectKey |

#### Team & User Tools (3 tools)
| Tool Name | Purpose | Required Params |
|-----------|---------|-----------------|
| `jira_list_users` | List project users | projectKey |
| `jira_get_user` | Get user details | accountId |
| `jira_search_users` | Search users | query |

#### Sprint & Board Tools (7 tools)
| Tool Name | Purpose | Required Params |
|-----------|---------|-----------------|
| `jira_list_boards` | List Scrum/Kanban boards | - |
| `jira_get_board` | Get board details | boardId |
| `jira_list_sprints` | List sprints | boardId |
| `jira_get_sprint` | Get sprint details | sprintId |
| `jira_move_to_sprint` | Add issues to sprint | issueKeys, sprintId |
| `jira_start_sprint` | Start sprint | sprintId |
| `jira_complete_sprint` | Complete sprint | sprintId |

#### Advanced Features (6 tools)
| Tool Name | Purpose | Required Params |
|-----------|---------|-----------------|
| `jira_create_filter` | Save JQL filter | name, jql |
| `jira_get_filter` | Get saved filter | filterId |
| `jira_bulk_update_issues` | Update multiple issues | issueKeys, fields |
| `jira_get_changelog` | Get issue history | issueKey |
| `jira_watch_issue` | Subscribe to notifications | issueKey |
| `jira_vote_issue` | Vote for issue | issueKey |

### Authentication Strategy

#### Option 1: API Token (Recommended for JIRA Cloud)
- **How it works**: Base64 encode `email:api_token` for Basic Auth
- **Security**: Token can be revoked from Atlassian account settings
- **Setup**: User generates token at https://id.atlassian.com/manage-profile/security/api-tokens
- **Implementation**:
  ```typescript
  const auth = Buffer.from(`${email}:${apiToken}`).toString('base64');
  headers['Authorization'] = `Basic ${auth}`;
  ```

#### Option 2: Personal Access Token (JIRA Data Center/Server)
- **How it works**: Bearer token in Authorization header
- **Security**: Token-based with scoped permissions
- **Implementation**:
  ```typescript
  headers['Authorization'] = `Bearer ${personalAccessToken}`;
  ```

#### Option 3: OAuth 2.0 (Advanced - Future Enhancement)
- **How it works**: Three-legged OAuth flow
- **Use case**: Distributed applications, third-party integrations
- **Complexity**: Requires callback URL, token refresh logic

**Recommendation**: Start with API Token (Option 1) for simplicity and security.

### Configuration Schema

```json
{
  "mcpServers": {
    "jira": {
      "command": "npx",
      "args": ["-y", "@janhq/jira-mcp-server"],
      "env": {
        "JIRA_URL": "https://your-domain.atlassian.net",
        "JIRA_EMAIL": "user@example.com",
        "JIRA_API_TOKEN": "your_api_token_here",
        "JIRA_DEFAULT_PROJECT": "PROJ"
      },
      "active": false,
      "official": true,
      "type": "stdio"
    }
  }
}
```

---

## Implementation Phases

### Phase 1: MCP Server Development (Weeks 1-2)

#### Week 1: Project Setup & Core Infrastructure

**Day 1-2: Repository Setup**
```bash
# Create new repository
mkdir jira-mcp-server
cd jira-mcp-server
npm init -y

# Install dependencies
npm install @modelcontextprotocol/sdk axios dotenv
npm install --save-dev typescript @types/node ts-node jest @types/jest

# Initialize TypeScript
npx tsc --init
```

**Project Structure**:
```
jira-mcp-server/
├── package.json
├── tsconfig.json
├── .env.example
├── README.md
├── src/
│   ├── index.ts              # Main server entry point
│   ├── jira-client.ts        # JIRA API client wrapper
│   ├── types.ts              # TypeScript type definitions
│   ├── handlers/             # Tool implementations
│   │   ├── index.ts
│   │   ├── issue-handlers.ts
│   │   ├── project-handlers.ts
│   │   ├── sprint-handlers.ts
│   │   └── user-handlers.ts
│   └── utils/
│       ├── validation.ts     # Input validation
│       ├── formatting.ts     # Response formatting
│       └── error-handling.ts # Error handling utilities
├── tests/
│   ├── jira-client.test.ts
│   └── handlers/
│       └── *.test.ts
└── docs/
    ├── API.md                # API documentation
    └── USAGE.md              # Usage examples
```

**Day 3-5: JIRA API Client Implementation**

```typescript
// src/jira-client.ts
import axios, { AxiosInstance } from 'axios';

export interface JiraConfig {
  url: string;
  email: string;
  apiToken: string;
}

export class JiraClient {
  private client: AxiosInstance;
  
  constructor(config: JiraConfig) {
    const auth = Buffer.from(`${config.email}:${config.apiToken}`).toString('base64');
    
    this.client = axios.create({
      baseURL: `${config.url}/rest/api/3`,
      headers: {
        'Authorization': `Basic ${auth}`,
        'Accept': 'application/json',
        'Content-Type': 'application/json',
      },
      timeout: 30000, // 30 second timeout
    });
    
    // Add response interceptor for error handling
    this.client.interceptors.response.use(
      response => response,
      error => this.handleError(error)
    );
  }
  
  private handleError(error: any): Promise<never> {
    if (error.response) {
      // JIRA API returned error
      const message = error.response.data?.errorMessages?.join(', ') 
        || error.response.data?.message 
        || error.message;
      throw new Error(`JIRA API Error: ${message}`);
    } else if (error.request) {
      // Network error
      throw new Error('Network error: Could not reach JIRA server');
    } else {
      throw new Error(`Request error: ${error.message}`);
    }
  }
  
  // Issue Management Methods
  async createIssue(data: CreateIssueParams): Promise<JiraIssue> {
    const response = await this.client.post('/issue', {
      fields: {
        project: { key: data.project },
        summary: data.summary,
        issuetype: { name: data.issueType },
        description: this.formatDescription(data.description),
        priority: data.priority ? { name: data.priority } : undefined,
        assignee: data.assignee ? { accountId: data.assignee } : undefined,
      },
    });
    return response.data;
  }
  
  async getIssue(issueKey: string): Promise<JiraIssue> {
    const response = await this.client.get(`/issue/${issueKey}`);
    return response.data;
  }
  
  async updateIssue(issueKey: string, fields: Partial<IssueFields>): Promise<void> {
    await this.client.put(`/issue/${issueKey}`, { fields });
  }
  
  async deleteIssue(issueKey: string): Promise<void> {
    await this.client.delete(`/issue/${issueKey}`);
  }
  
  async searchIssues(jql: string, maxResults = 50): Promise<JiraSearchResult> {
    const response = await this.client.post('/search', {
      jql,
      maxResults,
      fields: ['summary', 'status', 'assignee', 'priority', 'created', 'updated'],
    });
    return response.data;
  }
  
  async transitionIssue(issueKey: string, transitionId: string): Promise<void> {
    await this.client.post(`/issue/${issueKey}/transitions`, {
      transition: { id: transitionId },
    });
  }
  
  async addComment(issueKey: string, comment: string): Promise<JiraComment> {
    const response = await this.client.post(`/issue/${issueKey}/comment`, {
      body: this.formatDescription(comment),
    });
    return response.data;
  }
  
  // Project Methods
  async listProjects(): Promise<JiraProject[]> {
    const response = await this.client.get('/project');
    return response.data;
  }
  
  async getProject(projectKey: string): Promise<JiraProject> {
    const response = await this.client.get(`/project/${projectKey}`);
    return response.data;
  }
  
  // Workflow Methods
  async getTransitions(issueKey: string): Promise<JiraTransition[]> {
    const response = await this.client.get(`/issue/${issueKey}/transitions`);
    return response.data.transitions;
  }
  
  // JIRA uses Atlassian Document Format (ADF) for rich text
  private formatDescription(text?: string): any {
    if (!text) return undefined;
    
    return {
      type: 'doc',
      version: 1,
      content: [
        {
          type: 'paragraph',
          content: [
            {
              type: 'text',
              text: text,
            },
          ],
        },
      ],
    };
  }
  
  // Add more methods for sprints, boards, users, etc.
}

// Type definitions
export interface CreateIssueParams {
  project: string;
  summary: string;
  issueType: string;
  description?: string;
  priority?: string;
  assignee?: string;
}

export interface JiraIssue {
  id: string;
  key: string;
  fields: {
    summary: string;
    status: { name: string };
    issuetype: { name: string };
    priority?: { name: string };
    assignee?: { displayName: string; accountId: string };
    reporter?: { displayName: string; accountId: string };
    created: string;
    updated: string;
    description?: any;
  };
}

// Add more type definitions...
```

#### Week 2: MCP Server & Tool Handlers

**Day 6-7: MCP Server Core**

```typescript
// src/index.ts
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { JiraClient } from './jira-client.js';
import { validateEnvironment } from './utils/validation.js';
import { registerAllTools } from './handlers/index.js';

// Validate environment variables
validateEnvironment();

// Initialize JIRA client
const jiraClient = new JiraClient({
  url: process.env.JIRA_URL!,
  email: process.env.JIRA_EMAIL!,
  apiToken: process.env.JIRA_API_TOKEN!,
});

// Create MCP server
const server = new Server({
  name: 'jira-mcp-server',
  version: '1.0.0',
}, {
  capabilities: {
    tools: {},
  },
});

// Register all tools
registerAllTools(server, jiraClient);

// Start server
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error('JIRA MCP server running on stdio');
}

main().catch(error => {
  console.error('Fatal error:', error);
  process.exit(1);
});
```

**Day 8-10: Tool Handlers**

```typescript
// src/handlers/issue-handlers.ts
import { JiraClient } from '../jira-client.js';
import { formatIssueDetails, formatError } from '../utils/formatting.js';
import { validateIssueKey, validateProjectKey } from '../utils/validation.js';

export async function handleCreateIssue(args: any, client: JiraClient) {
  try {
    // Validate inputs
    if (!validateProjectKey(args.project)) {
      throw new Error('Invalid project key format');
    }
    
    const issue = await client.createIssue({
      project: args.project,
      summary: args.summary,
      issueType: args.issueType,
      description: args.description,
      priority: args.priority,
      assignee: args.assignee,
    });
    
    return {
      content: [
        {
          type: 'text',
          text: `✓ Successfully created issue ${issue.key}\n\n` +
                `Summary: ${args.summary}\n` +
                `Type: ${args.issueType}\n` +
                `URL: ${process.env.JIRA_URL}/browse/${issue.key}`,
        },
      ],
    };
  } catch (error) {
    return formatError(error, 'creating issue');
  }
}

export async function handleGetIssue(args: any, client: JiraClient) {
  try {
    if (!validateIssueKey(args.issueKey)) {
      throw new Error('Invalid issue key format. Expected format: PROJECT-123');
    }
    
    const issue = await client.getIssue(args.issueKey);
    
    return {
      content: [
        {
          type: 'text',
          text: formatIssueDetails(issue),
        },
      ],
    };
  } catch (error) {
    return formatError(error, 'fetching issue');
  }
}

export async function handleSearchIssues(args: any, client: JiraClient) {
  try {
    const result = await client.searchIssues(args.jql, args.maxResults || 50);
    
    const issueList = result.issues.map((issue: any) => 
      `${issue.key}: ${issue.fields.summary} [${issue.fields.status.name}]`
    ).join('\n');
    
    return {
      content: [
        {
          type: 'text',
          text: `Found ${result.total} issues (showing ${result.issues.length}):\n\n${issueList}`,
        },
      ],
    };
  } catch (error) {
    return formatError(error, 'searching issues');
  }
}

// Add more handlers...
```

### Phase 2: Jan Integration (Weeks 3-4)

#### Week 3: Backend Integration

**Task 1: Update Default Configuration**

File: `src-tauri/src/core/mcp/constants.rs`

```rust
pub const DEFAULT_MCP_CONFIG: &str = r#"{
  "mcpServers": {
    "Jan Browser MCP": {
      "command": "npx",
      "args": ["-y", "search-mcp-server@latest"],
      "env": {
        "BRIDGE_HOST": "127.0.0.1",
        "BRIDGE_PORT": "17389"
      },
      "active": false,
      "official": true
    },
    "jira": {
      "command": "npx",
      "args": ["-y", "@janhq/jira-mcp-server"],
      "env": {
        "JIRA_URL": "",
        "JIRA_EMAIL": "",
        "JIRA_API_TOKEN": "",
        "JIRA_DEFAULT_PROJECT": ""
      },
      "active": false,
      "official": true
    },
    "exa": {
      "command": "npx",
      "args": ["-y", "exa-mcp-server"],
      "env": { "EXA_API_KEY": "" },
      "active": false
    }
  },
  "mcpSettings": {
    "toolCallTimeoutSeconds": 30,
    "baseRestartDelayMs": 1000,
    "maxRestartDelayMs": 30000,
    "backoffMultiplier": 2.0,
    "proactiveMode": false
  }
}"#;
```

**Task 2: Add Migration Logic**

File: `src-tauri/src/core/setup.rs`

```rust
use crate::core::mcp::helpers::add_server_config;

pub fn migrate_mcp_servers_v3(
    app_handle: tauri::AppHandle,
    store: Arc<Store<Wry>>,
) -> Result<(), String> {
    let current_version = store
        .get("mcp_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    
    if current_version >= 3 {
        return Ok(()); // Already migrated
    }
    
    log::info!("Migrating MCP servers to version 3 (adding JIRA support)");
    
    // Add JIRA MCP server configuration if not present
    let result = add_server_config(
        app_handle,
        "JIRA".to_string(),
        serde_json::json!({
            "command": "npx",
            "args": ["-y", "@janhq/jira-mcp-server"],
            "env": {
                "JIRA_URL": "",
                "JIRA_EMAIL": "",
                "JIRA_API_TOKEN": "",
                "JIRA_DEFAULT_PROJECT": ""
            },
            "active": false,
            "official": true
        }),
    );
    
    if let Err(e) = result {
        log::error!("Failed to add JIRA MCP server config: {e}");
    } else {
        log::info!("Successfully added JIRA MCP server configuration");
    }
    
    // Update version
    store.set("mcp_version", 3);
    store.save().expect("Failed to save store");
    
    Ok(())
}
```

#### Week 4: Frontend Integration

**Task 1: Create JIRA Configuration Component**

File: `web-app/src/components/MCPServerConfig/JiraConfig.tsx`

```typescript
import React, { useState } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { ExternalLink, Eye, EyeOff } from 'lucide-react';
import { Callout } from '@/components/ui/callout';

interface JiraConfigProps {
  config: {
    JIRA_URL: string;
    JIRA_EMAIL: string;
    JIRA_API_TOKEN: string;
    JIRA_DEFAULT_PROJECT?: string;
  };
  onChange: (config: any) => void;
}

export const JiraConfig: React.FC<JiraConfigProps> = ({ config, onChange }) => {
  const [showToken, setShowToken] = useState(false);
  
  const handleTestConnection = async () => {
    // Add connection test logic
    try {
      // Call Tauri command to test JIRA connection
      console.log('Testing JIRA connection...');
    } catch (error) {
      console.error('Connection test failed:', error);
    }
  };
  
  return (
    <div className="space-y-4">
      <Callout type="info">
        You'll need a JIRA Cloud account and API token. Free tier available for small teams.
      </Callout>
      
      <div>
        <Label htmlFor="jira-url">JIRA URL *</Label>
        <Input
          id="jira-url"
          placeholder="https://your-domain.atlassian.net"
          value={config.JIRA_URL}
          onChange={(e) => onChange({ ...config, JIRA_URL: e.target.value })}
          required
        />
        <p className="text-sm text-muted-foreground mt-1">
          Your JIRA Cloud instance URL (found in your browser address bar)
        </p>
      </div>
      
      <div>
        <Label htmlFor="jira-email">Email *</Label>
        <Input
          id="jira-email"
          type="email"
          placeholder="user@example.com"
          value={config.JIRA_EMAIL}
          onChange={(e) => onChange({ ...config, JIRA_EMAIL: e.target.value })}
          required
        />
        <p className="text-sm text-muted-foreground mt-1">
          Email address associated with your JIRA account
        </p>
      </div>
      
      <div>
        <Label htmlFor="jira-token">API Token *</Label>
        <div className="relative">
          <Input
            id="jira-token"
            type={showToken ? 'text' : 'password'}
            placeholder="Your API token"
            value={config.JIRA_API_TOKEN}
            onChange={(e) => onChange({ ...config, JIRA_API_TOKEN: e.target.value })}
            required
          />
          <button
            type="button"
            className="absolute right-2 top-1/2 -translate-y-1/2"
            onClick={() => setShowToken(!showToken)}
          >
            {showToken ? <EyeOff size={16} /> : <Eye size={16} />}
          </button>
        </div>
        <Button
          variant="link"
          className="p-0 h-auto mt-1"
          onClick={() => window.open('https://id.atlassian.com/manage-profile/security/api-tokens', '_blank')}
        >
          Generate API Token <ExternalLink className="ml-1 h-3 w-3" />
        </Button>
      </div>
      
      <div>
        <Label htmlFor="jira-project">Default Project Key (Optional)</Label>
        <Input
          id="jira-project"
          placeholder="PROJ"
          value={config.JIRA_DEFAULT_PROJECT || ''}
          onChange={(e) => onChange({ ...config, JIRA_DEFAULT_PROJECT: e.target.value })}
          maxLength={10}
        />
        <p className="text-sm text-muted-foreground mt-1">
          Default project for quick issue creation (usually 2-10 uppercase letters)
        </p>
      </div>
      
      <Button onClick={handleTestConnection} variant="outline">
        Test Connection
      </Button>
    </div>
  );
};
```

**Task 2: Update MCP Server Settings Page**

Integrate the JIRA configuration component into the existing MCP server settings UI.

### Phase 3: Documentation & Testing (Week 5)

See separate sections below for detailed testing and documentation plans.

### Phase 4: Publishing & Release (Week 6)

See "Publishing & Distribution" section below.

---

## Testing & Quality Assurance

### Unit Tests

**Test Coverage Requirements**: Minimum 80% code coverage

#### JIRA Client Tests

File: `tests/jira-client.test.ts`

```typescript
import { JiraClient } from '../src/jira-client';

describe('JiraClient', () => {
  let client: JiraClient;
  
  beforeAll(() => {
    client = new JiraClient({
      url: process.env.JIRA_URL!,
      email: process.env.JIRA_EMAIL!,
      apiToken: process.env.JIRA_API_TOKEN!,
    });
  });
  
  describe('Issue Management', () => {
    test('should create an issue', async () => {
      const issue = await client.createIssue({
        project: 'TEST',
        summary: 'Test issue from MCP',
        issueType: 'Task',
        description: 'This is a test issue',
      });
      
      expect(issue.key).toMatch(/^TEST-\d+$/);
      expect(issue.fields.summary).toBe('Test issue from MCP');
    });
    
    test('should get an issue', async () => {
      const issue = await client.getIssue('TEST-1');
      expect(issue.fields.summary).toBeDefined();
      expect(issue.fields.status).toBeDefined();
    });
    
    test('should update an issue', async () => {
      await expect(client.updateIssue('TEST-1', {
        summary: 'Updated summary'
      })).resolves.not.toThrow();
    });
    
    test('should handle invalid issue key', async () => {
      await expect(client.getIssue('INVALID'))
        .rejects.toThrow('JIRA API Error');
    });
  });
  
  describe('Project Operations', () => {
    test('should list projects', async () => {
      const projects = await client.listProjects();
      expect(Array.isArray(projects)).toBe(true);
      expect(projects.length).toBeGreaterThan(0);
    });
    
    test('should get project details', async () => {
      const project = await client.getProject('TEST');
      expect(project.key).toBe('TEST');
      expect(project.name).toBeDefined();
    });
  });
  
  describe('Error Handling', () => {
    test('should handle network errors', async () => {
      const badClient = new JiraClient({
        url: 'https://invalid-url-that-does-not-exist.com',
        email: 'test@test.com',
        apiToken: 'invalid',
      });
      
      await expect(badClient.listProjects())
        .rejects.toThrow('Network error');
    });
    
    test('should handle authentication errors', async () => {
      const badClient = new JiraClient({
        url: process.env.JIRA_URL!,
        email: 'test@test.com',
        apiToken: 'invalid-token',
      });
      
      await expect(badClient.listProjects())
        .rejects.toThrow('JIRA API Error');
    });
  });
});
```

### Integration Tests

#### Jan Integration Tests

Add to `autoqa/checklist.md`:

```markdown
#### JIRA MCP Server Integration Tests

##### Server Lifecycle
- [ ] JIRA MCP server activates successfully with valid credentials
- [ ] Server shows clear error for invalid API token
- [ ] Server shows clear error for invalid JIRA URL
- [ ] Server auto-restarts on crash with exponential backoff
- [ ] Server stops cleanly when deactivated
- [ ] Multiple JIRA instances can be configured (e.g., work + personal)

##### Configuration
- [ ] Can save JIRA configuration through UI
- [ ] Configuration persists after app restart
- [ ] Can edit existing JIRA configuration
- [ ] Can delete JIRA configuration
- [ ] Environment variables are properly validated
- [ ] API token is never exposed in logs or UI

##### Tool Discovery & Calling
- [ ] All JIRA tools appear in tool list when server is active
- [ ] Tools disappear when server is deactivated
- [ ] Can call tools successfully with valid parameters
- [ ] Tool calls respect timeout settings (30s default)
- [ ] Tool call errors show user-friendly messages
- [ ] Can cancel long-running tool calls

##### Issue Management
- [ ] Can create issue with minimal fields (project, summary, type)
- [ ] Can create issue with all fields (description, priority, assignee)
- [ ] Can retrieve and display issue details correctly
- [ ] Can update issue fields
- [ ] Can add comments to issues
- [ ] Can search issues using JQL
- [ ] Can transition issue through workflow

##### Project & Workflow
- [ ] Can list all accessible projects
- [ ] Can get project details
- [ ] Can list issue types for a project
- [ ] Can get available transitions for an issue

##### Sprint & Agile
- [ ] Can list boards
- [ ] Can list sprints for a board
- [ ] Can add issues to sprint
- [ ] Can view sprint details

##### Permissions & Security
- [ ] Tool permission approval works correctly
- [ ] "Allow All MCP Tool Permissions" bypasses approvals
- [ ] Individual tool "Always Allow" works correctly
- [ ] Permissions reset properly for new threads

##### Error Handling
- [ ] Network errors show user-friendly messages
- [ ] JIRA API errors show helpful messages
- [ ] Rate limit errors handled gracefully
- [ ] Invalid parameters show validation errors

##### Performance
- [ ] Server starts in < 2 seconds
- [ ] Tool calls complete in < 1 second (excluding JIRA API time)
- [ ] Large result sets are paginated properly
- [ ] No memory leaks during extended use
```

### Security Testing

#### Security Checklist

- [ ] **Credential Storage**
  - API tokens stored only in `mcp_config.json`
  - Tokens never logged to console or files
  - Tokens not exposed in error messages
  - Tokens not sent to frontend except as masked values

- [ ] **Input Validation**
  - JQL injection prevention (parameterized queries)
  - XSS prevention in issue descriptions
  - Path traversal prevention in attachment operations
  - Project key validation (format: `^[A-Z]+$`)
  - Issue key validation (format: `^[A-Z]+-\d+$`)

- [ ] **API Security**
  - Rate limiting respected (JIRA Cloud: 10 req/sec per user)
  - Proper error handling for 401 (unauthorized)
  - Proper error handling for 403 (forbidden)
  - Proper error handling for 429 (rate limit)
  - HTTPS-only connections enforced

- [ ] **Data Privacy**
  - No sensitive data in logs
  - No credential exposure in stack traces
  - Proper sanitization of user inputs
  - No data leakage through error messages

---

## Scalability & Reusability

### MCP Server Template

Create a reusable template for future MCP servers:

**Repository**: `janhq/mcp-server-template`

```
mcp-server-template/
├── .github/
│   └── workflows/
│       ├── test.yml          # CI tests
│       └── publish.yml       # NPM publishing
├── package.json
├── tsconfig.json
├── .env.example
├── .gitignore
├── README.md
├── LICENSE
├── src/
│   ├── index.ts              # Main server entry point
│   ├── client.ts             # API client wrapper
│   ├── types.ts              # TypeScript types
│   ├── handlers/             # Tool implementations
│   │   ├── index.ts          # Handler registration
│   │   └── *.handler.ts      # Individual handlers
│   └── utils/
│       ├── validation.ts     # Input validation
│       ├── formatting.ts     # Response formatting
│       └── error-handling.ts # Error handling
├── tests/
│   ├── client.test.ts
│   ├── handlers/
│   │   └── *.test.ts
│   └── integration/
│       └── *.test.ts
└── docs/
    ├── API.md                # API reference
    ├── DEVELOPMENT.md        # Development guide
    └── USAGE.md              # Usage examples
```

### Shared Utilities Package

**Package**: `@janhq/mcp-utils`

```typescript
// Base server class
export abstract class MCPServerBase {
  protected server: Server;
  
  constructor(config: ServerConfig) {
    this.server = new Server(config.info, config.capabilities);
  }
  
  protected async registerTools(tools: ToolDefinition[]): Promise<void> {
    // Standard tool registration logic
  }
  
  protected handleError(error: Error, context: string): ToolResponse {
    // Standard error formatting
    return {
      content: [{
        type: 'text',
        text: `Error ${context}: ${error.message}`
      }],
      isError: true,
    };
  }
  
  protected formatResponse(data: any, template: string): ToolResponse {
    // Standard response formatting
  }
}

// Rate limiter utility
export class RateLimiter {
  private tokens: number;
  private lastRefill: number;
  private readonly maxTokens: number;
  private readonly refillRate: number; // tokens per second
  
  constructor(maxTokens: number, refillRate: number) {
    this.maxTokens = maxTokens;
    this.refillRate = refillRate;
    this.tokens = maxTokens;
    this.lastRefill = Date.now();
  }
  
  async acquire(): Promise<void> {
    this.refill();
    
    if (this.tokens >= 1) {
      this.tokens -= 1;
      return;
    }
    
    // Wait until token available
    const waitTime = (1 - this.tokens) / this.refillRate * 1000;
    await new Promise(resolve => setTimeout(resolve, waitTime));
    this.tokens = 0;
  }
  
  private refill(): void {
    const now = Date.now();
    const elapsed = (now - this.lastRefill) / 1000;
    this.tokens = Math.min(this.maxTokens, this.tokens + elapsed * this.refillRate);
    this.lastRefill = now;
  }
}

// Retry handler with exponential backoff
export class RetryHandler {
  async retry<T>(
    fn: () => Promise<T>,
    options: {
      maxAttempts?: number;
      baseDelay?: number;
      maxDelay?: number;
      multiplier?: number;
    } = {}
  ): Promise<T> {
    const {
      maxAttempts = 3,
      baseDelay = 1000,
      maxDelay = 30000,
      multiplier = 2,
    } = options;
    
    let lastError: Error;
    
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        return await fn();
      } catch (error) {
        lastError = error as Error;
        
        if (attempt === maxAttempts) {
          throw error;
        }
        
        const delay = Math.min(
          baseDelay * Math.pow(multiplier, attempt - 1),
          maxDelay
        );
        
        await new Promise(resolve => setTimeout(resolve, delay));
      }
    }
    
    throw lastError!;
  }
}
```

### Configuration Validation

```typescript
// config-validator.ts
import Ajv, { JSONSchemaType } from 'ajv';
import addFormats from 'ajv-formats';

interface MCPServerConfig {
  command: string;
  args: string[];
  env: Record<string, string>;
  active?: boolean;
  official?: boolean;
  type?: 'stdio' | 'http' | 'sse';
  url?: string;
  timeout?: number;
}

const mcpServerSchema: JSONSchemaType<MCPServerConfig> = {
  type: 'object',
  required: ['command', 'args', 'env'],
  properties: {
    command: { type: 'string', minLength: 1 },
    args: { type: 'array', items: { type: 'string' } },
    env: { type: 'object', required: [], additionalProperties: { type: 'string' } },
    active: { type: 'boolean', nullable: true },
    official: { type: 'boolean', nullable: true },
    type: { type: 'string', enum: ['stdio', 'http', 'sse'], nullable: true },
    url: { type: 'string', format: 'uri', nullable: true },
    timeout: { type: 'number', minimum: 1, nullable: true },
  },
};

export function validateMCPConfig(config: unknown): {
  valid: boolean;
  errors?: string[];
} {
  const ajv = new Ajv();
  addFormats(ajv);
  
  const validate = ajv.compile(mcpServerSchema);
  const valid = validate(config);
  
  if (!valid) {
    return {
      valid: false,
      errors: validate.errors?.map(err => `${err.instancePath} ${err.message}`) || [],
    };
  }
  
  return { valid: true };
}
```

---

## Publishing & Distribution

### NPM Package Structure

**package.json**:
```json
{
  "name": "@janhq/jira-mcp-server",
  "version": "1.0.0",
  "description": "JIRA MCP server for Jan AI assistant - manage issues, projects, and workflows through natural language",
  "main": "dist/index.js",
  "bin": {
    "jira-mcp-server": "dist/index.js"
  },
  "scripts": {
    "build": "tsc",
    "dev": "tsx watch src/index.ts",
    "test": "jest",
    "test:watch": "jest --watch",
    "lint": "eslint src tests",
    "prepublishOnly": "npm run build && npm test"
  },
  "keywords": [
    "mcp",
    "jira",
    "jan",
    "ai",
    "assistant",
    "model-context-protocol",
    "atlassian",
    "project-management"
  ],
  "author": "Jan Team <hello@jan.ai>",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/janhq/jira-mcp-server.git"
  },
  "bugs": {
    "url": "https://github.com/janhq/jira-mcp-server/issues"
  },
  "homepage": "https://jan.ai/docs/mcp-examples/productivity/jira",
  "dependencies": {
    "@modelcontextprotocol/sdk": "^0.5.0",
    "axios": "^1.6.0",
    "dotenv": "^16.0.0"
  },
  "devDependencies": {
    "@types/jest": "^29.5.0",
    "@types/node": "^20.0.0",
    "eslint": "^8.50.0",
    "jest": "^29.7.0",
    "ts-jest": "^29.1.0",
    "tsx": "^4.7.0",
    "typescript": "^5.3.0"
  },
  "engines": {
    "node": ">=18.0.0"
  }
}
```

### Publishing Checklist

**Pre-Publishing**:
- [ ] All tests passing (`npm test`)
- [ ] Code linted (`npm run lint`)
- [ ] README.md complete with examples
- [ ] API documentation written
- [ ] .env.example file created
- [ ] LICENSE file added (MIT)
- [ ] Security audit passed (`npm audit`)
- [ ] Version bumped (follow semver)
- [ ] CHANGELOG.md updated

**Publishing**:
- [ ] Build succeeds (`npm run build`)
- [ ] Package tested locally (`npm link`)
- [ ] Published to npm (`npm publish --access public`)
- [ ] Git tag created (`git tag v1.0.0`)
- [ ] GitHub release created with notes
- [ ] Documentation website updated

**Post-Publishing**:
- [ ] Test installation (`npx @janhq/jira-mcp-server`)
- [ ] Update Jan's official MCP registry
- [ ] Announce on Jan community channels
- [ ] Monitor for issues/feedback

### Official Server Registry

Add entry to Jan's MCP server registry:

**File**: `web-app/src/data/official-mcp-servers.json`

```json
{
  "servers": [
    {
      "id": "jira",
      "name": "JIRA",
      "description": "Manage JIRA issues, projects, and workflows through natural language",
      "category": "productivity",
      "package": "@janhq/jira-mcp-server",
      "version": "^1.0.0",
      "author": "Jan Team",
      "verified": true,
      "requiredEnv": ["JIRA_URL", "JIRA_EMAIL", "JIRA_API_TOKEN"],
      "optionalEnv": ["JIRA_DEFAULT_PROJECT"],
      "documentation": "https://jan.ai/docs/mcp-examples/productivity/jira",
      "repository": "https://github.com/janhq/jira-mcp-server",
      "license": "MIT",
      "tags": ["project-management", "issue-tracking", "agile", "atlassian"],
      "features": [
        "Create and manage issues",
        "Search with JQL",
        "Workflow transitions",
        "Sprint management",
        "Project navigation",
        "Team collaboration"
      ]
    }
  ]
}
```

---

## Risk Mitigation

### Technical Risks

| Risk | Impact | Probability | Mitigation Strategy |
|------|--------|-------------|---------------------|
| **JIRA API Breaking Changes** | High | Medium | - Pin to specific API version (v3)<br>- Comprehensive integration tests<br>- Monitor JIRA API changelog<br>- Version compatibility matrix |
| **Rate Limiting Issues** | Medium | High | - Implement client-side rate limiter (10 req/sec)<br>- Add request queuing<br>- Cache frequently accessed data<br>- Show rate limit status to users |
| **API Token Expiration** | High | Low | - Clear error messages on 401<br>- Token validation on startup<br>- Documentation on token refresh<br>- Auto-detect expired tokens |
| **Network Failures** | Medium | Medium | - Retry logic with exponential backoff<br>- Timeout handling (30s default)<br>- Offline mode detection<br>- Connection test utility |
| **Large Response Payloads** | Low | Medium | - Implement pagination<br>- Field filtering in API calls<br>- Response size limits<br>- Streaming for large datasets |
| **Memory Leaks** | Medium | Low | - Proper cleanup in error paths<br>- Connection pooling<br>- Memory profiling tests<br>- Resource monitoring |

### Security Risks

| Risk | Impact | Probability | Mitigation Strategy |
|------|--------|-------------|---------------------|
| **API Token Leakage** | Critical | Low | - Never log tokens<br>- Secure storage in config<br>- Masked display in UI<br>- Security audit of all code paths |
| **JQL Injection** | High | Medium | - Input validation<br>- Parameterized queries<br>- JQL sanitization<br>- Whitelist allowed characters |
| **XSS via Descriptions** | Medium | Low | - Content sanitization<br>- ADF format validation<br>- Escape HTML entities<br>- CSP headers |
| **Unauthorized Access** | High | Low | - Proper permission checks<br>- JIRA permission enforcement<br>- User context validation<br>- Audit logging |
| **Man-in-the-Middle** | High | Low | - HTTPS-only connections<br>- Certificate validation<br>- Reject invalid certs<br>- Network security audit |

### User Experience Risks

| Risk | Impact | Probability | Mitigation Strategy |
|------|--------|-------------|---------------------|
| **Complex Setup Process** | High | High | - Step-by-step UI wizard<br>- Visual validation feedback<br>- "Test Connection" button<br>- Video tutorials |
| **Tool Name Confusion** | Medium | Medium | - Clear, descriptive naming<br>- Tool categories in UI<br>- Inline documentation<br>- Search/filter tools |
| **Overwhelming Features** | Medium | High | - Progressive disclosure<br>- Sensible defaults<br>- Quick start templates<br>- Feature discovery hints |
| **Unclear Error Messages** | Medium | High | - User-friendly error formatting<br>- Actionable error messages<br>- Link to troubleshooting<br>- Error code reference |
| **Performance Perception** | Low | Medium | - Loading indicators<br>- Optimistic UI updates<br>- Background processing<br>- Response caching |

---

## Success Metrics

### Technical Metrics

**Performance**:
- ✓ Server start time: < 2 seconds (target: 1 second)
- ✓ Tool call latency: < 1 second (excluding JIRA API)
- ✓ Memory usage: < 100 MB baseline
- ✓ CPU usage: < 5% idle, < 50% under load
- ✓ Uptime: 99%+ with auto-restart

**Reliability**:
- ✓ Tool call success rate: > 99%
- ✓ Auto-restart success rate: > 95%
- ✓ Configuration validation accuracy: 100%
- ✓ Error handling coverage: 100% of error paths
- ✓ Test coverage: > 80%

**Security**:
- ✓ Zero credential leaks in logs/errors
- ✓ All inputs validated
- ✓ Security audit: Pass with no critical issues
- ✓ Dependency vulnerabilities: None (high/critical)

### User Metrics

**Adoption**:
- Active installations: Track via telemetry (opt-in)
- Daily active users: Target 100+ within 3 months
- Tool calls per user per day: Target 10+
- Retention rate: > 60% after 30 days

**Satisfaction**:
- Configuration success rate: > 95% first-time
- User satisfaction score: > 4.5/5
- Documentation clarity: > 4.0/5
- Support ticket volume: < 5% of users

**Engagement**:
- Feature utilization: > 70% of tools used at least once
- Power users (>20 calls/day): > 10%
- Integration combinations: Track which MCPs used together
- Community contributions: Issues, PRs, discussions

### Business Metrics

- NPM downloads: Track weekly/monthly
- GitHub stars: Target 100+ within 6 months
- Community engagement: Issues, PRs, discussions
- Documentation views: Track via analytics
- Social mentions: Monitor Twitter, Reddit, Discord

---

## Next Steps

### Immediate Actions (Week 0)

1. **Repository Setup**:
   - [ ] Create `janhq/jira-mcp-server` repository
   - [ ] Set up CI/CD pipelines (GitHub Actions)
   - [ ] Configure npm publishing workflow
   - [ ] Set up issue templates

2. **Environment Setup**:
   - [ ] Get JIRA Cloud test account
   - [ ] Generate API token for testing
   - [ ] Set up test JIRA projects
   - [ ] Document test environment setup

3. **Team Alignment**:
   - [ ] Review and approve implementation plan
   - [ ] Assign roles and responsibilities
   - [ ] Set up project tracking (GitHub Projects)
   - [ ] Schedule weekly sync meetings

### Phase Execution

Follow the implementation phases as outlined:
1. **Weeks 1-2**: MCP Server Development
2. **Weeks 3-4**: Jan Integration
3. **Week 5**: Documentation & Testing
4. **Week 6**: Publishing & Release

### Regular Reviews

- **Daily**: Quick standup (async or sync)
- **Weekly**: Phase progress review
- **Bi-weekly**: Demo to stakeholders
- **End of each phase**: Retrospective and adjustment

---

## Appendix

### Related Documentation

- **Jan MCP Architecture**: `.github/copilot-instructions.md` (MCP section)
- **Existing MCP Examples**: `docs/src/pages/docs/desktop/mcp-examples/`
- **Backend Implementation**: `src-tauri/src/core/mcp/`
- **Frontend Services**: `web-app/src/services/mcp/`
- **JIRA REST API v3**: https://developer.atlassian.com/cloud/jira/platform/rest/v3/

### Tool Quick Reference

| Category | Tool Count | Examples |
|----------|------------|----------|
| Issue Management | 12 | create, get, update, search, comment |
| Projects | 8 | list, get, issue types, components |
| Users | 3 | list, get, search |
| Sprints/Boards | 7 | list boards, manage sprints |
| Advanced | 6 | filters, changelog, watchers |
| **Total** | **36** | |

### Glossary

- **MCP**: Model Context Protocol - Standard for AI tool integration
- **JQL**: JIRA Query Language - SQL-like syntax for searching issues
- **ADF**: Atlassian Document Format - Rich text format used by JIRA
- **API Token**: Authentication credential for JIRA Cloud
- **PAT**: Personal Access Token - Auth for JIRA Data Center/Server
- **Tauri**: Framework for building desktop apps with web technologies
- **Zustand**: State management library for React

---

**Document Version**: 1.0  
**Last Updated**: December 4, 2025  
**Status**: Ready for Implementation  
**Next Review**: After Phase 1 completion
