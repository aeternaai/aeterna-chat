import { createFileRoute } from '@tanstack/react-router'
import { route } from '@/constants/routes'
import SettingsMenu from '@/containers/SettingsMenu'
import HeaderPage from '@/containers/HeaderPage'
import { useCallback, useEffect, useState } from 'react'
import { RouterManager } from '@janhq/core'
import { useAppState } from '@/hooks/useAppState'
import { useModelProvider } from '@/hooks/useModelProvider'
import { Switch } from '@/components/ui/switch'
import { Card, CardItem } from '@/containers/Card'
import { invoke } from '@tauri-apps/api/core'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const Route = createFileRoute(route.settings.router as any)({
  component: RouterSettings,
})

interface ModelRoutingConfig {
  id: string
  description: string
}

function RouterSettings() {
  const [strategies, setStrategies] = useState<
    Array<{ name: string; description: string }>
  >([])
  const [currentStrategy, setCurrentStrategy] = useState<string>('')
  const [loading, setLoading] = useState(true)
  const [allowedModels, setAllowedModels] = useState<string>('')
  const [routingConfigs, setRoutingConfigs] = useState<ModelRoutingConfig[]>([])
  const [editingConfigIndex, setEditingConfigIndex] = useState<number | null>(null)
  const [newConfig, setNewConfig] = useState<ModelRoutingConfig>({ id: '', description: '' })
  const [routerModel, setRouterModel] = useState<string>('')
  const routingEnabled = useAppState((state) => state.routingEnabled)
  const setRoutingEnabled = useAppState((state) => state.setRoutingEnabled)
  const getProviderByName = useModelProvider((state) => state.getProviderByName)
  
  // Get downloaded models from llamacpp provider
  const llamacppProvider = getProviderByName('llamacpp')
  const downloadedModels = llamacppProvider?.models || []

  useEffect(() => {
    loadRouterSettings()
  }, [])

  const loadRouterSettings = useCallback(async () => {
    try {
      console.log('[Router Settings] Attempting to load router...')
      
      // Load router model from Tauri
      try {
        const savedRouterModel = await invoke<string | null>('get_router_model_config')
        if (savedRouterModel) {
          setRouterModel(savedRouterModel)
          console.log('[Router Settings] Loaded router model:', savedRouterModel)
        }
      } catch (error) {
        console.error('[Router Settings] Failed to load router model:', error)
      }
      
      const routerManager = RouterManager.instance()
      console.log('[Router Settings] RouterManager instance:', routerManager)
      
      const router = routerManager.get()
      console.log('[Router Settings] Router from manager:', router)
      
      if (router) {
        const availableStrategies = router.listStrategies()
        setStrategies(availableStrategies)
        
        const activeStrategy = router.getStrategy()
        setCurrentStrategy(activeStrategy.name)

        // Load allowed models setting
        if (router.getSettings) {
          const settings = await router.getSettings()
          const allowedModelsSetting = settings.find(s => s.key === 'allowed_models')
          if (allowedModelsSetting) {
            setAllowedModels(allowedModelsSetting.controllerProps.value as string)
          }
          
          // Load model routing configs
          const routingConfigsSetting = settings.find(s => s.key === 'model_routing_configs')
          if (routingConfigsSetting) {
            try {
              const configs = JSON.parse(routingConfigsSetting.controllerProps.value as string)
              setRoutingConfigs(Array.isArray(configs) ? configs : [])
            } catch (err) {
              console.error('[Router Settings] Failed to parse routing configs:', err)
              setRoutingConfigs([])
            }
          }
        }

        console.log('[Router Settings] Loaded successfully:', {
          strategies: availableStrategies,
          currentStrategy: activeStrategy.name
        })
      } else {
        console.warn('[Router Settings] Router extension not yet loaded, retrying in 1000ms...')
        // Retry after a longer delay to allow extension loading
        setTimeout(() => {
          console.log('[Router Settings] Retry attempt...')
          if (!loading) {
            console.log('[Router Settings] Already loaded, skipping retry')
            return
          }
          const retryRouter = RouterManager.instance().get()
          console.log('[Router Settings] Retry - Router from manager:', retryRouter)
          
          if (retryRouter) {
            const availableStrategies = retryRouter.listStrategies()
            setStrategies(availableStrategies)
            
            const activeStrategy = retryRouter.getStrategy()
            setCurrentStrategy(activeStrategy.name)

            // Load allowed models setting on retry
            if (retryRouter.getSettings) {
              retryRouter.getSettings().then(settings => {
                const allowedModelsSetting = settings.find(s => s.key === 'allowed_models')
                if (allowedModelsSetting) {
                  setAllowedModels(allowedModelsSetting.controllerProps.value as string)
                }
                
                // Load model routing configs on retry
                const routingConfigsSetting = settings.find(s => s.key === 'model_routing_configs')
                if (routingConfigsSetting) {
                  try {
                    const configs = JSON.parse(routingConfigsSetting.controllerProps.value as string)
                    setRoutingConfigs(Array.isArray(configs) ? configs : [])
                  } catch (err) {
                    console.error('[Router Settings] Failed to parse routing configs:', err)
                    setRoutingConfigs([])
                  }
                }
              })
            }

            console.log('[Router Settings] Loaded successfully on retry:', {
              strategies: availableStrategies,
              currentStrategy: activeStrategy.name
            })
          } else {
            console.error('[Router Settings] Router still not available after retry')
          }
          setLoading(false)
        }, 1000)
        return // Don't set loading to false yet
      }
    } catch (error) {
      console.error('[Router Settings] Failed to load router settings:', error)
    } finally {
      setLoading(false)
    }
  }, [loading])

  const handleStrategyChange = useCallback(
    async (strategyName: string) => {
      try {
        const router = RouterManager.instance().get()
        if (router) {
          const success = router.setStrategyByName(strategyName)
          if (success) {
            setCurrentStrategy(strategyName)
            console.log(`[Router Settings] Switched to strategy: ${strategyName}`)
          }
        }
      } catch (error) {
        console.error('Failed to change strategy:', error)
      }
    },
    []
  )

  const handleToggleRouting = useCallback(() => {
    setRoutingEnabled(!routingEnabled)
  }, [routingEnabled, setRoutingEnabled])

  const handleRouterModelChange = useCallback(
    async (modelId: string) => {
      setRouterModel(modelId)
      try {
        await invoke('set_router_model_config', { routerModel: modelId || null })
        console.log('[Router Settings] Updated router model:', modelId)
      } catch (error) {
        console.error('[Router Settings] Failed to save router model:', error)
      }
    },
    []
  )

  const handleAllowedModelsChange = useCallback(
    async (value: string) => {
      setAllowedModels(value)
      try {
        const router = RouterManager.instance().get()
        if (router && router.updateSettings) {
          await router.updateSettings([
            { key: 'allowed_models', controllerProps: { value } }
          ])
          console.log('[Router Settings] Updated allowed models:', value)
        }
      } catch (error) {
        console.error('Failed to update allowed models:', error)
      }
    },
    []
  )

  const saveRoutingConfigs = useCallback(
    async (configs: ModelRoutingConfig[]) => {
      try {
        const router = RouterManager.instance().get()
        if (router && router.updateSettings) {
          const value = JSON.stringify(configs)
          await router.updateSettings([
            { key: 'model_routing_configs', controllerProps: { value } }
          ])
          console.log('[Router Settings] Updated routing configs:', configs)
        }
      } catch (error) {
        console.error('Failed to update routing configs:', error)
      }
    },
    []
  )

  const handleAddConfig = useCallback(() => {
    if (!newConfig.id.trim() || !newConfig.description.trim()) {
      return
    }
    
    const updatedConfigs = [...routingConfigs, { ...newConfig }]
    setRoutingConfigs(updatedConfigs)
    setNewConfig({ id: '', description: '' })
    saveRoutingConfigs(updatedConfigs)
  }, [newConfig, routingConfigs, saveRoutingConfigs])

  const handleEditConfig = useCallback((index: number) => {
    setEditingConfigIndex(index)
    setNewConfig({ ...routingConfigs[index] })
  }, [routingConfigs])

  const handleSaveEdit = useCallback(() => {
    if (editingConfigIndex === null || !newConfig.id.trim() || !newConfig.description.trim()) {
      return
    }
    
    const updatedConfigs = [...routingConfigs]
    updatedConfigs[editingConfigIndex] = { ...newConfig }
    setRoutingConfigs(updatedConfigs)
    setEditingConfigIndex(null)
    setNewConfig({ id: '', description: '' })
    saveRoutingConfigs(updatedConfigs)
  }, [editingConfigIndex, newConfig, routingConfigs, saveRoutingConfigs])

  const handleCancelEdit = useCallback(() => {
    setEditingConfigIndex(null)
    setNewConfig({ id: '', description: '' })
  }, [])

  const handleDeleteConfig = useCallback((index: number) => {
    const updatedConfigs = routingConfigs.filter((_, i) => i !== index)
    setRoutingConfigs(updatedConfigs)
    saveRoutingConfigs(updatedConfigs)
  }, [routingConfigs, saveRoutingConfigs])

  if (loading) {
    return (
      <div className="flex flex-col h-full">
        <HeaderPage>
          <h1 className="font-medium">Settings</h1>
        </HeaderPage>
        <div className="flex h-full w-full flex-col sm:flex-row">
          <SettingsMenu />
          <div className="p-4 w-full h-[calc(100%-32px)] overflow-y-auto">
            <div className="flex items-center justify-center py-8">
              <div className="text-main-view-fg/60">Loading router settings...</div>
            </div>
          </div>
        </div>
      </div>
    )
  }

  if (strategies.length === 0) {
    return (
      <div className="flex flex-col h-full">
        <HeaderPage>
          <h1 className="font-medium">Settings</h1>
        </HeaderPage>
        <div className="flex h-full w-full flex-col sm:flex-row">
          <SettingsMenu />
          <div className="p-4 w-full h-[calc(100%-32px)] overflow-y-auto">
            <div className="flex flex-col items-center justify-center py-8 space-y-2">
              <div className="text-main-view-fg/60">Router extension not loaded</div>
              <div className="text-xs text-main-view-fg/40">
                The router extension may not be installed or enabled
              </div>
            </div>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full pb-[calc(env(safe-area-inset-bottom)+env(safe-area-inset-top))]">
      <HeaderPage>
        <h1 className="font-medium">Settings</h1>
      </HeaderPage>
      <div className="flex h-full w-full flex-col sm:flex-row">
        <SettingsMenu />
        <div className="p-4 w-full h-[calc(100%-32px)] overflow-y-auto">
          <div className="flex flex-col justify-between gap-4 gap-y-3 w-full">
            {/* Enable/Disable Auto Routing */}
            <Card title="Auto Routing">
              <CardItem
                title="Enable Auto Routing"
                description="Automatically select the best model for each query based on content analysis"
                className="flex-col sm:flex-row items-start sm:items-center sm:justify-between gap-y-2"
                actions={
                  <Switch
                    checked={routingEnabled}
                    onCheckedChange={handleToggleRouting}
                  />
                }
              />
            </Card>

            {/* Strategy Selection */}
            <Card title="Routing Strategy">
              <div className="p-4 flex flex-col space-y-3">
                <p className="text-xs text-main-view-fg/60">
                  Choose how the router analyzes queries and selects models
                </p>

                <div className="space-y-2">
                  {strategies.map((strategy) => (
                    <div
                      key={strategy.name}
                      onClick={() => handleStrategyChange(strategy.name)}
                      className={`
                        flex items-start p-4 rounded-lg border cursor-pointer transition-all
                        ${
                          currentStrategy === strategy.name
                            ? 'border-primary bg-primary/5'
                            : 'border-main-view-fg/10 hover:border-main-view-fg/20 hover:bg-main-view-fg/5'
                        }
                      `}
                    >
                      <div className="flex items-start flex-1 min-w-0">
                        <div className="flex-shrink-0 mt-0.5">
                          <div
                            className={`
                              w-4 h-4 rounded-full border-2 flex items-center justify-center
                              ${
                                currentStrategy === strategy.name
                                  ? 'border-primary'
                                  : 'border-main-view-fg/30'
                              }
                            `}
                          >
                            {currentStrategy === strategy.name && (
                              <div className="w-2 h-2 rounded-full bg-primary"></div>
                            )}
                          </div>
                        </div>
                        <div className="ml-3 flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="text-sm font-medium text-main-view-fg capitalize">
                              {strategy.name.replace('-', ' ')}
                            </span>
                            {currentStrategy === strategy.name && (
                              <span className="text-xs text-primary font-medium">
                                Active
                              </span>
                            )}
                          </div>
                          <p className="text-xs text-main-view-fg/60 mt-1">
                            {strategy.description}
                          </p>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </Card>

            {/* Router Model Selection (for LLM-based strategy) */}
            <Card title="Router Model">
              <CardItem
                title="LLM Router Model"
                description="Select which model to use for LLM-based routing. This model analyzes your query to decide which response model to use. Smaller, faster models work best."
                className="flex-col items-start gap-y-2"
              />
              <div className="px-4 pb-4">
                <select
                  value={routerModel}
                  onChange={(e) => handleRouterModelChange(e.target.value)}
                  className="w-full px-3 py-2 text-sm border border-main-view-fg/10 rounded-lg bg-transparent text-main-view-fg focus:outline-none focus:border-primary"
                >
                  <option value="">-- Select Router Model --</option>
                  {downloadedModels.map((model) => (
                    <option key={model.id} value={model.id}>
                      {model.id}
                    </option>
                  ))}
                </select>
                <p className="text-xs text-main-view-fg/60 mt-2">
                  The router model is used only for the LLM-based routing strategy. It should be a small, fast model like Phi-4-mini.
                  {routerModel && (
                    <span className="block mt-1 text-primary">
                      Current: {routerModel}
                    </span>
                  )}
                </p>
              </div>
            </Card>

            {/* Allowed Models Configuration */}
            <Card title="Allowed Models">
              <CardItem
                title="Model Whitelist"
                description="Comma-separated list of model IDs that the router is allowed to select. Leave empty to allow all models."
                className="flex-col sm:flex-row items-start gap-y-2"
              />
              <div className="px-4 pb-4">
                <input
                  type="text"
                  value={allowedModels}
                  onChange={(e) => handleAllowedModelsChange(e.target.value)}
                  placeholder="e.g., Qwen3-VL-8B-Instruct-IQ4_XS,gemma-3n-E4B-it-IQ4_XS"
                  className="w-full px-3 py-2 text-sm border border-main-view-fg/10 rounded-lg bg-transparent text-main-view-fg focus:outline-none focus:border-primary"
                />
                <p className="text-xs text-main-view-fg/60 mt-2">
                  Example: <code className="px-1 py-0.5 bg-main-view-fg/5 rounded">Qwen3-VL-8B-Instruct-IQ4_XS,gemma-3n-E4B-it-IQ4_XS</code>
                </p>
              </div>
            </Card>

            {/* Model Routing Configuration */}
            <Card title="Model Routing Configuration">
              <CardItem
                title="Configure Model Selection"
                description="Define when each model should be used by the LLM-based router. Add descriptions that help the router understand which model to select for different types of queries."
                className="flex-col items-start gap-y-2"
              />
              
              {/* Existing configurations list */}
              {routingConfigs.length > 0 && (
                <div className="px-4 pb-2">
                  <div className="space-y-2">
                    {routingConfigs.map((config, index) => (
                      <div
                        key={index}
                        className="p-3 border border-main-view-fg/10 rounded-lg bg-main-view-fg/5"
                      >
                        {editingConfigIndex === index ? (
                          // Edit mode
                          <div className="space-y-2">
                            <div>
                              <label className="text-xs text-main-view-fg/60 block mb-1">
                                Model ID
                              </label>
                              <input
                                type="text"
                                value={newConfig.id}
                                onChange={(e) =>
                                  setNewConfig({ ...newConfig, id: e.target.value })
                                }
                                placeholder="e.g., Qwen3-VL-8B-Instruct-IQ4_XS"
                                className="w-full px-2 py-1.5 text-sm border border-main-view-fg/10 rounded bg-transparent text-main-view-fg focus:outline-none focus:border-primary"
                              />
                            </div>
                            <div>
                              <label className="text-xs text-main-view-fg/60 block mb-1">
                                When to use this model
                              </label>
                              <textarea
                                value={newConfig.description}
                                onChange={(e) =>
                                  setNewConfig({ ...newConfig, description: e.target.value })
                                }
                                placeholder="Describe when the router should select this model..."
                                rows={3}
                                className="w-full px-2 py-1.5 text-sm border border-main-view-fg/10 rounded bg-transparent text-main-view-fg focus:outline-none focus:border-primary resize-none"
                              />
                            </div>
                            <div className="flex gap-2 justify-end">
                              <button
                                onClick={handleCancelEdit}
                                className="px-3 py-1 text-xs border border-main-view-fg/10 rounded hover:bg-main-view-fg/5 text-main-view-fg transition-colors"
                              >
                                Cancel
                              </button>
                              <button
                                onClick={handleSaveEdit}
                                disabled={!newConfig.id.trim() || !newConfig.description.trim()}
                                className="px-3 py-1 text-xs bg-primary text-white rounded hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                              >
                                Save
                              </button>
                            </div>
                          </div>
                        ) : (
                          // View mode
                          <div className="flex items-start justify-between gap-3">
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2 mb-1">
                                <code className="text-xs font-mono text-primary">
                                  {config.id}
                                </code>
                              </div>
                              <p className="text-xs text-main-view-fg/70">
                                {config.description}
                              </p>
                            </div>
                            <div className="flex gap-1 flex-shrink-0">
                              <button
                                onClick={() => handleEditConfig(index)}
                                className="px-2 py-1 text-xs border border-main-view-fg/10 rounded hover:bg-main-view-fg/10 text-main-view-fg transition-colors"
                                title="Edit configuration"
                              >
                                Edit
                              </button>
                              <button
                                onClick={() => handleDeleteConfig(index)}
                                className="px-2 py-1 text-xs border border-red-500/20 rounded hover:bg-red-500/10 text-red-500 transition-colors"
                                title="Delete configuration"
                              >
                                Delete
                              </button>
                            </div>
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Add new configuration form */}
              {editingConfigIndex === null && (
                <div className="px-4 pb-4">
                  <div className="p-3 border border-dashed border-main-view-fg/20 rounded-lg space-y-2">
                    <div>
                      <label className="text-xs text-main-view-fg/60 block mb-1">
                        Model ID
                      </label>
                      <input
                        type="text"
                        value={newConfig.id}
                        onChange={(e) =>
                          setNewConfig({ ...newConfig, id: e.target.value })
                        }
                        placeholder="e.g., Qwen3-VL-8B-Instruct-IQ4_XS"
                        className="w-full px-2 py-1.5 text-sm border border-main-view-fg/10 rounded bg-transparent text-main-view-fg focus:outline-none focus:border-primary"
                      />
                    </div>
                    <div>
                      <label className="text-xs text-main-view-fg/60 block mb-1">
                        When to use this model
                      </label>
                      <textarea
                        value={newConfig.description}
                        onChange={(e) =>
                          setNewConfig({ ...newConfig, description: e.target.value })
                        }
                        placeholder="Vision and image understanding tasks. Use for analyzing images, describing visual content, OCR, and any query involving pictures or visual data."
                        rows={3}
                        className="w-full px-2 py-1.5 text-sm border border-main-view-fg/10 rounded bg-transparent text-main-view-fg focus:outline-none focus:border-primary resize-none"
                      />
                    </div>
                    <div className="flex justify-end">
                      <button
                        onClick={handleAddConfig}
                        disabled={!newConfig.id.trim() || !newConfig.description.trim()}
                        className="px-3 py-1.5 text-xs bg-primary text-white rounded hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                      >
                        Add Model Configuration
                      </button>
                    </div>
                  </div>
                  <p className="text-xs text-main-view-fg/60 mt-2">
                    💡 Tip: Write clear, specific descriptions that explain when this model should be selected. The LLM router uses these descriptions to make intelligent routing decisions.
                  </p>
                </div>
              )}
            </Card>

            {/* Info Section */}
            <Card title="How Auto Routing Works">
              <div className="p-4">
                <ul className="text-xs text-main-view-fg/60 space-y-1.5 list-disc list-inside">
                  <li>
                    When enabled, the router automatically selects the best model for
                    each message
                  </li>
                  <li>
                    Code queries are routed to models with code generation
                    capabilities
                  </li>
                  <li>
                    Complex reasoning tasks are sent to larger, more capable models
                  </li>
                  <li>
                    Simple queries use faster, smaller models for quick responses
                  </li>
                  <li>
                    You can override automatic routing by manually selecting a model
                  </li>
                </ul>
              </div>
            </Card>

            {/* Strategy Descriptions */}
            <Card title="Strategy Details">
              <div className="p-4 space-y-3">
                <div>
                  <h5 className="text-xs font-medium text-main-view-fg">
                    Heuristic Strategy
                  </h5>
                  <p className="text-xs text-main-view-fg/60 mt-1">
                    Fast rule-based routing using pattern matching and keyword
                    analysis. Routing decisions complete in &lt;10ms with no
                    additional API calls.
                  </p>
                </div>

                <div>
                  <h5 className="text-xs font-medium text-main-view-fg">
                    LLM-Based Strategy
                  </h5>
                  <p className="text-xs text-main-view-fg/60 mt-1">
                    Uses a small language model to analyze queries and make
                    intelligent routing decisions. More accurate but requires
                    loading an additional model. (Coming soon)
                  </p>
                </div>
              </div>
            </Card>
          </div>
        </div>
      </div>
    </div>
  )
}
