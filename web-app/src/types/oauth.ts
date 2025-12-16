/**
 * OAuth Types for MCP Server Authentication
 */

export interface OAuthConfig {
  client_id: string
  auth_url: string
  token_url: string
  scopes: string[]
  redirect_uri?: string
}

export interface OAuthToken {
  access_token: string
  refresh_token?: string
  expires_at: number
}

export interface OAuthStatus {
  authenticated: boolean
  expires_at?: number
  server_name: string
}

export interface OAuthFlowResult {
  authUrl: string
}
