/**
 * Hook for managing MCP server OAuth authentication
 */

import { useState, useEffect, useCallback } from 'react'
import { listen } from '@tauri-apps/api/event'
import { toast } from 'sonner'
import { getServiceHub } from '@/hooks/useServiceHub'
import type { OAuthStatus } from '@/types/oauth'

export interface UseMCPOAuthReturn {
  oauthStatuses: Record<string, OAuthStatus>
  isLoading: boolean
  startOAuthFlow: (serverName: string, oauthConfig: {
    client_id: string
    auth_url: string
    token_url: string
    scopes: string[]
    redirect_uri?: string
  }) => Promise<void>
  revokeOAuthToken: (serverName: string) => Promise<void>
  refreshStatuses: () => Promise<void>
  getServerStatus: (serverName: string) => OAuthStatus | null
  isAuthenticated: (serverName: string) => boolean
}

export function useMCPOAuth(): UseMCPOAuthReturn {
  const [oauthStatuses, setOAuthStatuses] = useState<Record<string, OAuthStatus>>({})
  const [isLoading, setIsLoading] = useState(false)

  // Load all OAuth statuses
  const refreshStatuses = useCallback(async () => {
    try {
      const statuses = await getServiceHub().mcp().getAllOAuthStatuses()
      setOAuthStatuses(statuses)
    } catch (error) {
      console.error('Failed to load OAuth statuses:', error)
    }
  }, [])

  // Initialize OAuth statuses on mount
  useEffect(() => {
    refreshStatuses()
  }, [refreshStatuses])

  // Listen for OAuth completion events
  useEffect(() => {
    const unlisten = listen<{ server_name: string; authenticated: boolean; expires_at?: number }>(
      'mcp_oauth_complete',
      (event) => {
        const { server_name, authenticated, expires_at } = event.payload
        
        // Update the status for this server
        setOAuthStatuses(prev => ({
          ...prev,
          [server_name]: {
            server_name,
            authenticated,
            expires_at,
          },
        }))

        // Show success toast
        toast.success(`Successfully authenticated with ${server_name}`)
      }
    )

    return () => {
      unlisten.then(fn => fn())
    }
  }, [])

  // Listen for OAuth revocation events
  useEffect(() => {
    const unlisten = listen<{ server_name: string }>(
      'mcp_oauth_revoked',
      (event) => {
        const { server_name } = event.payload
        
        // Update the status for this server
        setOAuthStatuses(prev => ({
          ...prev,
          [server_name]: {
            server_name,
            authenticated: false,
          },
        }))

        // Show info toast
        toast.info(`OAuth token revoked for ${server_name}`)
      }
    )

    return () => {
      unlisten.then(fn => fn())
    }
  }, [])

  // Start OAuth flow for a server
  const startOAuthFlow = useCallback(async (serverName: string, oauthConfig: {
    client_id: string
    auth_url: string
    token_url: string
    scopes: string[]
    redirect_uri?: string
  }) => {
    setIsLoading(true)
    try {
      console.log('[OAuth] Starting flow for server:', serverName)
      console.log('[OAuth] OAuth config:', oauthConfig)
      
      // Request OAuth flow start from backend
      const result = await getServiceHub().mcp().startOAuthFlow(serverName, oauthConfig)
      
      console.log('[OAuth] Received result from backend:', result)
      console.log('[OAuth] Auth URL:', result.authUrl)
      console.log('[OAuth] Result type:', typeof result)
      console.log('[OAuth] Result keys:', Object.keys(result || {}))
      
      // Open the authorization URL in browser
      if (result && result.authUrl) {
        console.log('[OAuth] Opening browser with URL:', result.authUrl)
        const opened = window.open(result.authUrl, '_blank')
        console.log('[OAuth] Window.open result:', opened)
        
        if (!opened) {
          toast.error('Failed to open browser. Please check popup blocker settings.')
        } else {
          toast.info(`Please complete authentication in your browser for ${serverName}`)
        }
      } else {
        console.error('[OAuth] No authUrl in result:', result)
        toast.error('Failed to get authorization URL from server')
      }
    } catch (error) {
      console.error('[OAuth] Failed to start OAuth flow:', error)
      toast.error(`Failed to start OAuth flow: ${error}`)
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Revoke OAuth token for a server
  const revokeOAuthToken = useCallback(async (serverName: string) => {
    setIsLoading(true)
    try {
      await getServiceHub().mcp().revokeOAuthToken(serverName)
      toast.success(`OAuth token revoked for ${serverName}`)
    } catch (error) {
      console.error('Failed to revoke OAuth token:', error)
      toast.error(`Failed to revoke OAuth token: ${error}`)
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Get OAuth status for a specific server
  const getServerStatus = useCallback((serverName: string): OAuthStatus | null => {
    return oauthStatuses[serverName] || null
  }, [oauthStatuses])

  // Check if a server is authenticated
  const isAuthenticated = useCallback((serverName: string): boolean => {
    const status = oauthStatuses[serverName]
    return status?.authenticated ?? false
  }, [oauthStatuses])

  return {
    oauthStatuses,
    isLoading,
    startOAuthFlow,
    revokeOAuthToken,
    refreshStatuses,
    getServerStatus,
    isAuthenticated,
  }
}
