/**
 * Hook for managing MCP server OAuth authentication
 */

import { useState, useEffect, useCallback } from 'react'
import { listen } from '@tauri-apps/api/event'
import { openUrl } from '@tauri-apps/plugin-opener'
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
  clearMcpRemoteAuth: () => Promise<void>
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
      console.log('[OAuth Debug Frontend] Fetching OAuth statuses...')
      const statuses = await getServiceHub().mcp().getAllOAuthStatuses()
      console.log('[OAuth Debug Frontend] Received OAuth statuses:', statuses)
      console.log('[OAuth Debug Frontend] Number of statuses:', Object.keys(statuses).length)
      Object.entries(statuses).forEach(([name, status]) => {
        console.log(`[OAuth Debug Frontend] ${name}: authenticated=${status.authenticated}`)
      })
      setOAuthStatuses(statuses)
    } catch (error) {
      console.error('Failed to load OAuth statuses:', error)
    }
  }, [])

  // Initialize OAuth statuses on mount
  useEffect(() => {
    refreshStatuses()
  }, [refreshStatuses])

  // Listen for OAuth required events (when backend detects OAuth prompt from mcp-remote)
  useEffect(() => {
    // Track recently opened URLs to prevent duplicates
    const recentlyOpened = new Map<string, number>()
    const DEBOUNCE_MS = 5000 // Don't open same URL within 5 seconds
    
    const unlisten = listen<{ server: string; url: string }>(
      'mcp_oauth_required',
      async (event) => {
        const { server, url } = event.payload
        console.log(`[OAuth] Backend detected OAuth required for ${server}, opening URL:`, url)
        
        // Check if we recently opened this URL
        const lastOpened = recentlyOpened.get(url)
        const now = Date.now()
        if (lastOpened && (now - lastOpened) < DEBOUNCE_MS) {
          console.log(`[OAuth] URL for ${server} was recently opened (${now - lastOpened}ms ago), skipping duplicate`)
          return
        }
        
        // Record this opening
        recentlyOpened.set(url, now)
        
        try {
          await openUrl(url)
          console.log(`[OAuth] Successfully opened OAuth URL for ${server}`)
          toast.info(`Please complete authentication in your browser for ${server}`)
        } catch (error) {
          console.error(`[OAuth] Failed to open OAuth URL for ${server}:`, error)
          toast.error(`Failed to open browser for ${server}. Please try again.`)
        }
      }
    )

    return () => {
      unlisten.then(fn => fn())
    }
  }, [])

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

  // Start OAuth flow for a server (triggers mcp-remote OAuth by clearing credentials and restarting)
  const startOAuthFlow = useCallback(async (serverName: string, _oauthConfig: {
    client_id: string
    auth_url: string
    token_url: string
    scopes: string[]
    redirect_uri?: string
  }) => {
    setIsLoading(true)
    try {
      console.log('[OAuth] Starting OAuth flow for server:', serverName)
      console.log('[OAuth] Triggering mcp-remote authentication by clearing credentials and restarting server')
      
      // Clear mcp-remote credentials
      await getServiceHub().mcp().clearMcpRemoteAuth()
      console.log('[OAuth] Cleared mcp-remote credentials')
      
      // Deactivate the server
      await getServiceHub().mcp().deactivateMCPServer(serverName)
      console.log('[OAuth] Deactivated server:', serverName)
      
      // Wait a moment for clean shutdown
      await new Promise(resolve => setTimeout(resolve, 500))
      
      // Get the server config to reactivate it
      const config = await getServiceHub().mcp().getMCPConfig()
      const serverConfig = config.mcpServers?.[serverName]
      
      if (!serverConfig) {
        throw new Error(`Server configuration not found for ${serverName}`)
      }
      
      // Reactivate the server - this will trigger mcp-remote to detect missing auth and prompt
      await getServiceHub().mcp().activateMCPServer(serverName, {
        ...serverConfig,
        active: true
      })
      console.log('[OAuth] Reactivated server:', serverName)
      
      toast.info(`Authenticating ${serverName}. The browser will open automatically.`)
    } catch (error) {
      console.error('[OAuth] Failed to trigger OAuth flow:', error)
      toast.error(`Failed to start authentication: ${error}`)
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

  // Clear MCP remote auth folder
  const clearMcpRemoteAuth = useCallback(async () => {
    setIsLoading(true)
    try {
      await getServiceHub().mcp().clearMcpRemoteAuth()
      toast.success('MCP remote authentication credentials cleared')
    } catch (error) {
      console.error('Failed to clear MCP remote auth:', error)
      toast.error(`Failed to clear credentials: ${error}`)
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
    const result = status?.authenticated ?? false
    console.log(`[OAuth Debug Frontend] isAuthenticated(${serverName}):`, {
      hasStatus: !!status,
      status,
      result
    })
    return result
  }, [oauthStatuses])

  return {
    oauthStatuses,
    isLoading,
    startOAuthFlow,
    revokeOAuthToken,
    clearMcpRemoteAuth,
    refreshStatuses,
    getServerStatus,
    isAuthenticated,
  }
}
