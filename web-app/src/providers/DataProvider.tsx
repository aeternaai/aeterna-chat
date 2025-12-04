import { useModelProvider } from '@/hooks/useModelProvider'

import { useAppUpdater } from '@/hooks/useAppUpdater'
import { useServiceHub } from '@/hooks/useServiceHub'
import { useEffect } from 'react'
import { useMCPServers, DEFAULT_MCP_SETTINGS } from '@/hooks/useMCPServers'
import { useAssistant } from '@/hooks/useAssistant'
import { useNavigate } from '@tanstack/react-router'
import { route } from '@/constants/routes'
import { useThreads } from '@/hooks/useThreads'
import { useLocalApiServer } from '@/hooks/useLocalApiServer'
import { useAppState } from '@/hooks/useAppState'
import { AppEvent, events } from '@janhq/core'
import { SystemEvent } from '@/types/events'
import { getModelToStart } from '@/utils/getModelToStart'
import { isPlatformTauri } from '@/lib/platform'

export function DataProvider() {
  const { setProviders, selectedModel, selectedProvider, getProviderByName } =
    useModelProvider()

  const { checkForUpdate } = useAppUpdater()
  const { setServers, setSettings } = useMCPServers()
  const { setAssistants, initializeWithLastUsed } = useAssistant()
  const { setThreads } = useThreads()
  const navigate = useNavigate()
  const serviceHub = useServiceHub()
  const setActiveModels = useAppState((state) => state.setActiveModels)
  const serverStatus = useAppState((state) => state.serverStatus)

  // Local API Server hooks
  const {
    enableOnStartup,
    serverHost,
    serverPort,
    setServerPort,
    apiPrefix,
    apiKey,
    trustedHosts,
    corsEnabled,
    verboseLogs,
    proxyTimeout,
  } = useLocalApiServer()
  const setServerStatus = useAppState((state) => state.setServerStatus)

  useEffect(() => {
    console.log('Initializing DataProvider...')
    serviceHub.providers().getProviders().then(setProviders)
    serviceHub
      .mcp()
      .getMCPConfig()
      .then((data) => {
        setServers(data.mcpServers ?? {})
        setSettings(data.mcpSettings ?? DEFAULT_MCP_SETTINGS)
      })
    serviceHub
      .assistants()
      .getAssistants()
      .then((data) => {
        // Only update assistants if we have valid data
        if (data && Array.isArray(data) && data.length > 0) {
          setAssistants(data as unknown as Assistant[])
          initializeWithLastUsed()
        }
      })
      .catch((error) => {
        console.warn('Failed to load assistants, keeping default:', error)
      })
    serviceHub.deeplink().getCurrent().then(handleDeepLink)
    serviceHub.deeplink().onOpenUrl(handleDeepLink)

    // Listen for deep link events
    let unsubscribe = () => {}
    serviceHub
      .events()
      .listen(SystemEvent.DEEP_LINK, (event) => {
        const deep_link = event.payload as string
        handleDeepLink([deep_link])
      })
      .then((unsub) => {
        unsubscribe = unsub
      })
    return () => {
      unsubscribe()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [serviceHub])

  useEffect(() => {
    serviceHub
      .threads()
      .fetchThreads()
      .then((threads) => {
        setThreads(threads)
      })
  }, [serviceHub, setThreads])

  // Check for app updates
  useEffect(() => {
    // Only check for updates if the auto updater is not disabled
    // App might be distributed via other package managers
    // or methods that handle updates differently
    if (!AUTO_UPDATER_DISABLED) {
      checkForUpdate()
    }
  }, [checkForUpdate])

  useEffect(() => {
    events.on(AppEvent.onModelImported, () => {
      serviceHub.providers().getProviders().then(setProviders)
    })
  }, [serviceHub, setProviders])

  // Keep Python router LLM config in sync with local API server settings
  // Only configure when server is actually running
  useEffect(() => {
    if (!isPlatformTauri()) return
    if (serverStatus !== 'running') return // Wait for server to be running
    if (!serverHost || !serverPort) return

    const prefix = apiPrefix?.startsWith('/') ? apiPrefix : `/${apiPrefix ?? ''}`
    const sanitizedPrefix = prefix.replace(/\/+$/, '')
    const apiKeyPayload = apiKey && apiKey.toString().trim().length > 0 ? apiKey : undefined
    if (!apiKeyPayload) return

    const baseUrl = `http://${serverHost}:${serverPort}${sanitizedPrefix}`

    console.log(`[DataProvider] Configuring router with baseUrl: ${baseUrl}, model: Phi-4-mini-instruct_Q4_K_M`)

    serviceHub
      .core()
      .invoke('configure_router_llm', {
        config: {
          baseUrl,
          apiKey: apiKeyPayload,
          model: 'Phi-4-mini-instruct_Q4_K_M', // Dedicated lightweight router model
        },
      })
      .then(() => {
        console.log('[DataProvider] Router LLM config updated successfully')
      })
      .catch((error) => {
        console.warn('[DataProvider] Failed to configure Python router LLM settings:', error)
      })
  }, [serviceHub, serverHost, serverPort, apiPrefix, apiKey, serverStatus])

  // Auto-load router model for LLM-based routing
  // Only load after server is running to ensure it's accessible
  useEffect(() => {
    if (!isPlatformTauri()) return
    if (serverStatus !== 'running') return // Wait for server to be running

    const ROUTER_MODEL_ID = 'Phi-4-mini-instruct_Q4_K_M'
    const llamacppProvider = getProviderByName('llamacpp')

    if (!llamacppProvider) {
      console.warn('[DataProvider] Cannot load router model: llamacpp provider not available')
      return
    }

    // Check if router model exists in the provider
    const routerModelExists = llamacppProvider.models?.some((m) => m.id === ROUTER_MODEL_ID)
    if (!routerModelExists) {
      console.warn(
        `[DataProvider] Router model '${ROUTER_MODEL_ID}' not found in llamacpp provider. ` +
          'Please download it to enable LLM-based routing.'
      )
      return
    }

    // Check if router model is already loaded
    serviceHub
      .models()
      .getActiveModels()
      .then((activeModels) => {
        const isRouterModelLoaded = activeModels?.includes(ROUTER_MODEL_ID)

        if (isRouterModelLoaded) {
          console.log(`[DataProvider] Router model '${ROUTER_MODEL_ID}' is already loaded`)
          return
        }

        // Load the router model in the background
        console.log(`[DataProvider] Loading router model '${ROUTER_MODEL_ID}' for LLM-based routing...`)
        return serviceHub.models().startModel(llamacppProvider, ROUTER_MODEL_ID)
      })
      .then(() => {
        console.log(`[DataProvider] Router model '${ROUTER_MODEL_ID}' loaded successfully`)
        
        // Mark this session as the router session to exempt from auto-unload
        // This is critical if using the same model for both routing and answering
        if (llamacppProvider && typeof (llamacppProvider as any).setRouterSession === 'function') {
          ;(llamacppProvider as any).setRouterSession(ROUTER_MODEL_ID).catch((error: Error) => {
            console.warn('[DataProvider] Failed to mark router session:', error)
          })
        }
      })
      .catch((error) => {
        console.warn(`[DataProvider] Failed to load router model '${ROUTER_MODEL_ID}':`, error)
      })
  }, [serviceHub, getProviderByName, serverStatus])

  // Auto-start Local API Server on app startup if enabled
  useEffect(() => {
    if (enableOnStartup) {
      // Validate API key before starting
      if (!apiKey || apiKey.toString().trim().length === 0) {
        console.warn('Cannot start Local API Server: API key is required')
        return
      }

      const modelToStart = getModelToStart({
        selectedModel,
        selectedProvider,
        getProviderByName,
      })

      // Only start server if we have a model to load
      if (!modelToStart) {
        console.warn(
          'Cannot start Local API Server: No model available to load'
        )
        return
      }

      setServerStatus('pending')

      // Start the model first
      serviceHub
        .models()
        .startModel(modelToStart.provider, modelToStart.model)
        .then(() => {
          console.log(`Model ${modelToStart.model} started successfully`)
          // Refresh active models after starting
          serviceHub
            .models()
            .getActiveModels()
            .then((models) => setActiveModels(models || []))

          // Then start the server
          return window.core?.api?.startServer({
            host: serverHost,
            port: serverPort,
            prefix: apiPrefix,
            apiKey,
            trustedHosts,
            isCorsEnabled: corsEnabled,
            isVerboseEnabled: verboseLogs,
            proxyTimeout: proxyTimeout,
          })
        })
        .then((actualPort: number) => {
          // Store the actual port that was assigned (important for mobile with port 0)
          if (actualPort && actualPort !== serverPort) {
            setServerPort(actualPort)
          }
          setServerStatus('running')
        })
        .catch((error: unknown) => {
          console.error('Failed to start Local API Server on startup:', error)
          setServerStatus('stopped')
        })
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [serviceHub])

  const handleDeepLink = (urls: string[] | null) => {
    if (!urls) return
    console.log('Received deeplink:', urls)
    const deeplink = urls[0]
    if (deeplink) {
      const url = new URL(deeplink)
      const params = url.pathname.split('/').filter((str) => str.length > 0)

      if (params.length < 3) return undefined
      // const action = params[0]
      // const provider = params[1]
      const resource = params.slice(1).join('/')
      // return { action, provider, resource }
      navigate({
        to: route.hub.model,
        search: {
          repo: resource,
        },
      })
    }
  }

  return null
}
