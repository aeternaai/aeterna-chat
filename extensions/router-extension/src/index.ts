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
  private allowedModels: string[] = []
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

    // Load allowed models from settings
    const allowedModelsStr = await this.getSetting<string>(
      'allowed_models',
      'Qwen3-VL-8B-Instruct-IQ4_XS,gemma-3n-E4B-it-IQ4_XS'
    )
    this.allowedModels = this.parseAllowedModels(allowedModelsStr)
    console.log('[RouterExtension] Allowed models:', this.allowedModels)

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

    // Filter by allowed models
    const filteredModels = this.filterAllowedModels(context.availableModels)

    const filteredContext = {
      ...context,
      availableModels: filteredModels,
    }

    if (filteredContext.availableModels.length === 0) {
      console.error('[RouterExtension] No models available after filtering by allowed models')
      throw new Error('No suitable models available for routing. Please check your allowed models configuration.')
    }

    // Try Python router first if available
    if (this.usePythonRouter && this.pythonRouterAvailable) {
      try {
        console.log(`🐍 [RouterExtension] Using PYTHON router service`)
        const startTime = Date.now()
        
        // Call Python router via Tauri command
        const decision = await invoke('route_request', {
          request: {
            messages: filteredContext.messages,
            threadId: filteredContext.threadId,
            availableModels: filteredContext.availableModels,
            activeModels: filteredContext.activeModels,
            attachments: filteredContext.attachments,
            preferences: filteredContext.preferences,
          }
        }) as RouteDecision
        
        const elapsed = Date.now() - startTime
        
        const selectedModel = filteredModels.find(m => m.id === decision.modelId)
        const needsLoading = selectedModel && !selectedModel.metadata.isLoaded

        console.log(
          `[RouterExtension] Python router: ${decision.modelId} (${elapsed}ms) - ${decision.reasoning}${needsLoading ? ' [will be loaded]' : ' [already loaded]'}`
        )

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

  private parseAllowedModels(allowedModelsStr: string): string[] {
    if (!allowedModelsStr || allowedModelsStr.trim() === '') {
      return []
    }
    return allowedModelsStr
      .split(',')
      .map((id) => id.trim())
      .filter((id) => id.length > 0)
  }

  private filterAllowedModels(availableModels: any[]): any[] {
    // If no allowed models configured, return all available models
    if (this.allowedModels.length === 0) {
      return availableModels
    }

    // Filter models to only include those in the allowed list
    return availableModels.filter((model) => this.allowedModels.includes(model.id))
  }

  onSettingUpdate<T>(key: string, value: T): void {
    if (key === 'allowed_models') {
      this.allowedModels = this.parseAllowedModels(value as string)
      console.log('[RouterExtension] Updated allowed models:', this.allowedModels)
    }
  }
}
