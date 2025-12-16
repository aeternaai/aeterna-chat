/**
 * MCP Service Types
 */

import { MCPTool, MCPToolCallResult } from '@janhq/core'
import type { MCPServerConfig, MCPServers, MCPSettings } from '@/hooks/useMCPServers'
import type { OAuthStatus, OAuthFlowResult } from '@/types/oauth'

export interface MCPConfig {
  mcpServers?: MCPServers
  mcpSettings?: MCPSettings
}

export interface ToolCallWithCancellationResult {
  promise: Promise<MCPToolCallResult>
  cancel: () => Promise<void>
  token: string
}

export interface MCPService {
  updateMCPConfig(configs: string): Promise<void>
  restartMCPServers(): Promise<void>
  getMCPConfig(): Promise<MCPConfig>
  getTools(): Promise<MCPTool[]>
  getConnectedServers(): Promise<string[]>
  callTool(args: { toolName: string; serverName?: string; arguments: object }): Promise<MCPToolCallResult>
  callToolWithCancellation(args: {
    toolName: string
    serverName?: string
    arguments: object
    cancellationToken?: string
  }): ToolCallWithCancellationResult
  cancelToolCall(cancellationToken: string): Promise<void>

  // MCP Server lifecycle management
  activateMCPServer(name: string, config: MCPServerConfig): Promise<void>
  deactivateMCPServer(name: string): Promise<void>

  // OAuth methods
  startOAuthFlow(serverName: string, oauthConfig: {
    client_id: string
    auth_url: string
    token_url: string
    scopes: string[]
    redirect_uri?: string
  }): Promise<OAuthFlowResult>
  getOAuthStatus(serverName: string): Promise<OAuthStatus>
  getAllOAuthStatuses(): Promise<Record<string, OAuthStatus>>
  revokeOAuthToken(serverName: string): Promise<void>
}
