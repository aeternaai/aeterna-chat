import {
  ModelRouterExtension,
  RouterStrategy,
  RouteContext,
  RouteDecision,
  RouterManager,
  fs,
  joinPath,
} from '@janhq/core'
import { invoke } from '@tauri-apps/api/core'
import { HeuristicRouter } from './strategies/HeuristicRouter'
import { LLMRouter } from './strategies/LLMRouter'

/**
 * Check if running in Tauri context by attempting to use invoke
 */
async function isTauriContext(): Promise<boolean> {
  try {
    // If invoke exists and works, we're in Tauri
    await invoke('get_router_health')
    return true
  } catch (error) {
    // Either not in Tauri, or Python router not ready
    // This is expected in web mode or during startup
    return false
  }
}

/**
 * Router Extension that delegates to Python router service via Tauri commands
 * Falls back to TypeScript heuristic routing if Python service is unavailable
 */
export default class RouterExtension extends ModelRouterExtension {
  private activeStrategy: RouterStrategy
  private availableStrategies: Map<string, RouterStrategy>
  private modelRoutingConfigs: Array<{id: string, description: string}> = []
  private usePythonRouter: boolean = true
  private pythonRouterAvailable: boolean = false

  constructor(url: string, name: string, productName?: string) {
    super(url, name, productName, true, 'Model Router Extension', '1.0.0')

    // Initialize fallback TypeScript strategies
    this.availableStrategies = new Map<string, RouterStrategy>([
      ['heuristic', new HeuristicRouter()],
      ['llm-based', new LLMRouter()],
    ])

    // Default to heuristic (fastest)
    this.activeStrategy = this.availableStrategies.get('heuristic')!
  }

  async onLoad() {
    console.log('[RouterExtension] Loading model router')

    // Register settings
    const settings = structuredClone(SETTINGS)
    await this.registerSettings(settings)

    // Load routing configs from settings
    const routingConfigsStr = await this.getSetting<string>(
      'model_routing_configs',
      '[{"id":"Qwen3-VL-8B-Instruct-IQ4_XS","description":"Vision and image understanding tasks. Use for analyzing images, describing visual content, OCR, and any query involving pictures or visual data."},{"id":"gemma-3n-E4B-it-IQ4_XS","description":"Coding, programming, and technical documentation tasks. Use for writing code, debugging, explaining technical concepts, and software development."}]'
    )
    this.modelRoutingConfigs = this.parseRoutingConfigs(routingConfigsStr)
    console.log('[RouterExtension] Model routing configs:', this.modelRoutingConfigs)

    // Register with RouterManager
    const routerManager = typeof window !== 'undefined' && window.core?.routerManager 
      ? window.core.routerManager 
      : RouterManager.instance()
    
    routerManager.register(this)
    console.log('[RouterExtension] Registered with RouterManager:', routerManager)

    // Check if Python router service is available (Tauri only)
    // Use retry logic to allow time for service startup during app initialization
    console.log('[RouterExtension] 🚀 Checking for Python router...')
    this.pythonRouterAvailable = await this.checkPythonRouterWithRetry(5, 1000)

    // Load user preferences for routing strategy
    const savedStrategy = await this.loadStrategyPreference()
    if (savedStrategy && this.availableStrategies.has(savedStrategy)) {
      this.activeStrategy = this.availableStrategies.get(savedStrategy)!
    }

    console.log(`[RouterExtension] Active strategy: ${this.activeStrategy.name}`)
    
    // Sync strategy to Python router if available
    if (this.usePythonRouter && this.pythonRouterAvailable) {
      await this.syncStrategyToPythonRouter(this.activeStrategy.name).catch(error => {
        console.warn(`[RouterExtension] Failed to sync strategy to Python router:`, error)
        // Continue with TypeScript fallback
      })
    }
    
    // Clear debug output showing which router will be used
    if (this.usePythonRouter && this.pythonRouterAvailable) {
      console.log(`✅ [RouterExtension] ROUTER MODE: PYTHON (Python service available on port 8765)`)
    } else if (this.usePythonRouter && !this.pythonRouterAvailable) {
      console.log(`⚠️  [RouterExtension] ROUTER MODE: TYPESCRIPT FALLBACK (Python service not available)`)
    } else {
      console.log(`📜 [RouterExtension] ROUTER MODE: TYPESCRIPT (Python router disabled)`)
    }
  }

  async onUnload() {
    console.log('[RouterExtension] Unloading model router')
  }

  getStrategy(): RouterStrategy {
    return this.activeStrategy
  }

  setStrategy(strategy: RouterStrategy): void {
    this.activeStrategy = strategy
    this.saveStrategyPreference(strategy.name)
  }

  setStrategyByName(name: string): boolean {
    if (this.availableStrategies.has(name)) {
      this.activeStrategy = this.availableStrategies.get(name)!
      this.saveStrategyPreference(name)
      
      // If using Python router, sync the strategy asynchronously
      if (this.usePythonRouter && this.pythonRouterAvailable) {
        this.syncStrategyToPythonRouter(name).catch(error => {
          console.warn(`[RouterExtension] Failed to sync strategy to Python router:`, error)
        })
      }
      
      return true
    }
    return false
  }
  
  /**
   * Sync strategy to Python router service (async helper)
   * 
   * Note: If the Python router doesn't support the strategy, it will fail gracefully
   * and routing will fall back to TypeScript implementation
   */
  private async syncStrategyToPythonRouter(strategyName: string): Promise<void> {
    try {
      await invoke('set_router_strategy', { strategyName })
      console.log(`[RouterExtension] ✅ Synced strategy to Python router: ${strategyName}`)
    } catch (error: any) {
      // Check if it's a "strategy not found" error
      const errorStr = error?.toString() || ''
      if (errorStr.includes('not found') || errorStr.includes('404')) {
        console.warn(`[RouterExtension] ⚠️  Strategy '${strategyName}' not available in Python router, will use TypeScript fallback`)
      } else {
        // Different error - rethrow
        throw error
      }
    }
  }

  listStrategies(): Array<{ name: string; description: string }> {
    return Array.from(this.availableStrategies.values()).map((s) => ({
      name: s.name,
      description: s.description,
    }))
  }

  async route(context: RouteContext): Promise<RouteDecision> {
    console.log(`[RouterExtension] Routing request...`)
    console.log(`[RouterExtension] Input available response models (${context.availableModels.length}):`, context.availableModels.map(m => m.id))
    console.log(`[RouterExtension] Router model:`, context.routerModel?.id || 'none')
    console.log(`[RouterExtension] Model routing configs (${this.modelRoutingConfigs.length}):`, this.modelRoutingConfigs)

    // Filter by routing configs - only applies to RESPONSE models
    const filteredModels = this.filterAllowedModels(context.availableModels)
    console.log(`[RouterExtension] After filtering (${filteredModels.length}):`, filteredModels.map(m => m.id))
    
    // Create filtered context with response models only (router model passed separately)
    const filteredContext = {
      ...context,
      availableModels: filteredModels,
    }

    console.log(`[RouterExtension] Final available response models (${filteredContext.availableModels.length}):`, filteredContext.availableModels.map(m => m.id))

    if (filteredContext.availableModels.length === 0) {
      console.error('[RouterExtension] No response models available after filtering by allowed models')
      throw new Error('No suitable models available for routing. Please check your allowed models configuration.')
    }

    // Try Python router first if available
    if (this.usePythonRouter && this.pythonRouterAvailable) {
      try {
        console.log(`🐍 [RouterExtension] Using PYTHON router service`)
        console.log(`🐍 [RouterExtension] Router model:`, context.routerModel?.id || 'not provided')
        console.log(`🐍 [RouterExtension] Sending ${filteredContext.availableModels.length} response models to Python router:`)
        filteredContext.availableModels.forEach((m, idx) => {
          console.log(`   ${idx + 1}. ${m.id} (${m.providerId}) - ${m.capabilities.join(', ')}`)
        })
        
        const startTime = Date.now()
        
        // Call Python router via Tauri command, passing router model and routing configs
        const decision = await invoke('route_request', {
          request: {
            messages: filteredContext.messages,
            threadId: filteredContext.threadId,
            availableModels: filteredContext.availableModels,
            routerModel: filteredContext.routerModel,
            activeModels: filteredContext.activeModels,
            attachments: filteredContext.attachments,
            preferences: filteredContext.preferences,
            modelRoutingConfigs: this.modelRoutingConfigs,
          }
        }) as RouteDecision
        
        const elapsed = Date.now() - startTime
        
        const selectedModel = filteredModels.find(m => m.id === decision.modelId)
        const needsLoading = selectedModel && !selectedModel.metadata.isLoaded

        console.log(
          `[RouterExtension] Python router selected: ${decision.modelId} (index would be ${filteredContext.availableModels.findIndex(m => m.id === decision.modelId) + 1}) - ${decision.reasoning}${needsLoading ? ' [will be loaded]' : ' [already loaded]'}`
        )
        console.log(`[RouterExtension] Decision metadata:`, decision.metadata)

        this.logRoutingDecision(decision, elapsed, context)
        
        return decision
      } catch (error) {
        console.error('[RouterExtension] Python router failed, falling back to TypeScript:', error)
        // Fall through to TypeScript router
      }
    }

    // Fallback to TypeScript router
    console.log(`📜 [RouterExtension] Using TYPESCRIPT router (strategy: ${this.activeStrategy.name})`)
    
    const startTime = Date.now()
    const decision = await this.activeStrategy.route(filteredContext)
    const elapsed = Date.now() - startTime

    const selectedModel = filteredModels.find(m => m.id === decision.modelId)
    const needsLoading = selectedModel && !selectedModel.metadata.isLoaded

    console.log(
      `[RouterExtension] Routed to ${decision.modelId} (${elapsed}ms) - ${decision.reasoning}${needsLoading ? ' [will be loaded]' : ' [already loaded]'}`
    )

    this.logRoutingDecision(decision, elapsed, context)

    return decision
  }

  private async loadStrategyPreference(): Promise<string | null> {
    try {
      // Check if settings file exists first
      const settingsPath = await joinPath(['file://settings', 'router.json'])
      if (!(await fs.existsSync(settingsPath))) {
        return null
      }
      
      // Load from settings
      const settings = await fs.readFileSync(settingsPath)
      const parsed = JSON.parse(settings)
      return parsed?.strategy || null
    } catch (error) {
      // Error reading or parsing file
      return null
    }
  }

  private async saveStrategyPreference(strategyName: string): Promise<void> {
    try {
      // Ensure settings directory exists
      const settingsDir = 'file://settings'
      if (!(await fs.existsSync(settingsDir))) {
        await fs.mkdir(settingsDir)
      }
      
      // Save to settings
      const settingsPath = await joinPath([settingsDir, 'router.json'])
      await fs.writeFileSync(settingsPath, JSON.stringify({ strategy: strategyName }))
    } catch (error) {
      console.error('[RouterExtension] Failed to save strategy preference:', error)
    }
  }

  private logRoutingDecision(
    decision: RouteDecision,
    elapsed: number,
    context: RouteContext
  ): void {
    // Could send to analytics, store in DB, etc.
    const logEntry = {
      timestamp: Date.now(),
      strategy: this.activeStrategy.name,
      decision,
      elapsed,
      queryLength:
        this.getMessageContent(context.messages[context.messages.length - 1]).length || 0,
    }

    // For now, just console log
    console.debug('[RouterExtension] Decision:', logEntry)
  }

  /**
   * Check if Python router is available with retry logic
   * Allows time for Python service to start during app initialization
   */
  private async checkPythonRouterWithRetry(
    maxAttempts: number,
    delayMs: number
  ): Promise<boolean> {
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        await invoke('get_router_health')
        if (attempt > 1) {
          console.log(
            `[RouterExtension] 🎯 Python router ready (attempt ${attempt}/${maxAttempts})`
          )
        }
        return true
      } catch (error) {
        if (attempt === maxAttempts) {
          console.warn(
            `[RouterExtension] ⏱️  Python router not available after ${maxAttempts} attempts (${maxAttempts * delayMs}ms total)`,
            error
          )
          return false
        }
        
        // Log retry attempts (but not the first one to reduce noise)
        if (attempt > 1) {
          console.log(
            `[RouterExtension] ⏳ Waiting for Python router... (attempt ${attempt}/${maxAttempts})`
          )
        }
        
        // Wait before retrying
        await new Promise((resolve) => setTimeout(resolve, delayMs))
      }
    }
    return false
  }

  private getMessageContent(message: any): string {
    if (typeof message.content === 'string') {
      return message.content
    }
    if (Array.isArray(message.content)) {
      return message.content
        .filter((item) => item.type === 'text')
        .map((item) => item.text)
        .join(' ')
    }
    return ''
  }

  private parseRoutingConfigs(configsStr: string): Array<{id: string, description: string}> {
    if (!configsStr || configsStr.trim() === '') {
      return []
    }
    try {
      const configs = JSON.parse(configsStr)
      if (!Array.isArray(configs)) {
        console.warn('[RouterExtension] Invalid routing configs format, expected array:', configsStr)
        return []
      }
      return configs.filter((config) => 
        config && typeof config.id === 'string' && typeof config.description === 'string'
      )
    } catch (error) {
      console.error('[RouterExtension] Failed to parse routing configs:', error)
      return []
    }
  }

  private filterAllowedModels(availableModels: any[]): any[] {
    // If no routing configs configured, return all available models
    if (this.modelRoutingConfigs.length === 0) {
      return availableModels
    }

    // Filter models to only include those with routing configs
    const allowedIds = this.modelRoutingConfigs.map(config => config.id)
    return availableModels.filter((model) => allowedIds.includes(model.id))
  }

  onSettingUpdate<T>(key: string, value: T): void {
    if (key === 'model_routing_configs') {
      this.modelRoutingConfigs = this.parseRoutingConfigs(value as string)
      console.log('[RouterExtension] Updated model routing configs:', this.modelRoutingConfigs)
    }
  }

  /**
   * Get the list of allowed model IDs from routing configs
   * @returns Array of model IDs that are allowed for routing
   */
  getAllowedModelIds(): string[] {
    return this.modelRoutingConfigs.map(config => config.id)
  }
}
