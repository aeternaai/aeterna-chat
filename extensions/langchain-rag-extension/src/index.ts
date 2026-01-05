/**
 * LangChain RAG Extension
 * 
 * Provides LangChain-powered RAG capabilities as an alternative to the default RAG system.
 * Uses the tauri-plugin-langchain for Python LangChain integration.
 */

import { 
  RAGExtension,
  MCPTool, 
  MCPToolCallResult, 
  ExtensionTypeEnum,
  type AttachmentInput, 
  type SettingComponentProps,
  type AttachmentFileInfo,
  WorkspaceEvent,
  events,
  type WorkspaceFile
} from '@janhq/core'
import { invoke } from '@tauri-apps/api/core'
import './env.d'

// LangChain plugin types
interface ServiceConfig {
  call_timeout_ms: number
  auto_restart: boolean
  max_restart_attempts: number
  restart_count: number
  restart_delay_ms: number
  health_check_interval_ms: number
}

interface IngestParams {
  file_path: string
  collection: string
  chunk_size?: number
  chunk_overlap?: number
  metadata?: Record<string, unknown>
}

interface IngestResponse {
  collection: string
  chunks_count: number
  document_ids: string[]
  source_file: string
}

interface QueryParams {
  query: string
  collection: string
  k?: number
  score_threshold?: number
  filter?: Record<string, unknown>
}

interface QueryResponse {
  answer: string
  sources: Array<{
    content: string
    metadata: Record<string, unknown>
    score: number
  }>
  num_sources: number
  query: string
  collection: string
  processing_time_ms: number
}

export default class LangChainRagExtension extends RAGExtension {
  type(): ExtensionTypeEnum | undefined {
    return ExtensionTypeEnum.LangChainRAG
  }

  private config = {
    enabled: true,  // Enabled by default
    llm_server_url: 'http://127.0.0.1:8080/v1',
    embeddings_model: 'sentence-transformers/all-MiniLM-L6-v2',
    retrieval_limit: 3,
    chunk_size: 512,
    chunk_overlap: 64,
    llm_temperature: 0.7,
    llm_max_tokens: 1024,
  }

  private serviceStarted = false
  private ingestionInProgress = new Set<string>()

  async onLoad(): Promise<void> {
    console.log('[LangChain RAG] ========== onLoad() ENTRY ==========')
    console.log('[LangChain RAG] Extension loading...')
    console.log('[LangChain RAG] Extension type:', this.type())
    
    // Try to register settings, but don't fail if SETTINGS is not available
    try {
      const settings = structuredClone(SETTINGS) as SettingComponentProps[]
      await this.registerSettings(settings)
      console.log('[LangChain RAG] Settings registered successfully')
    } catch (error) {
      console.warn('[LangChain RAG] Failed to register settings:', error)
      console.log('[LangChain RAG] Proceeding without registered settings')
    }
    
    // Load configuration with defaults
    try {
      this.config.enabled = await this.getSetting('enabled', this.config.enabled)
      this.config.llm_server_url = await this.getSetting('llm_server_url', this.config.llm_server_url)
      this.config.embeddings_model = await this.getSetting('embeddings_model', this.config.embeddings_model)
      this.config.retrieval_limit = await this.getSetting('retrieval_limit', this.config.retrieval_limit)
      this.config.chunk_size = await this.getSetting('chunk_size', this.config.chunk_size)
      this.config.chunk_overlap = await this.getSetting('chunk_overlap', this.config.chunk_overlap)
      this.config.llm_temperature = await this.getSetting('llm_temperature', this.config.llm_temperature)
      this.config.llm_max_tokens = await this.getSetting('llm_max_tokens', this.config.llm_max_tokens)
      console.log('[LangChain RAG] Configuration loaded:', {
        enabled: this.config.enabled,
        retrieval_limit: this.config.retrieval_limit,
      })
    } catch (error) {
      console.warn('[LangChain RAG] Error loading settings, using defaults:', error)
    }

    if (this.config.enabled) {
      console.log('[LangChain RAG] Extension enabled, starting service...')
      try {
        await this.ensureServiceRunning()
        // Setup workspace file listener for RAG indexing
        this.setupWorkspaceFileListener()
        console.log('[LangChain RAG] Service started and listeners setup')
      } catch (error) {
        console.error('[LangChain RAG] Failed to start service during onLoad:', error)
      }
    } else {
      console.log('[LangChain RAG] Extension disabled in settings')
    }
    console.log('[LangChain RAG] ========== onLoad() COMPLETE ==========')
  }

  onUnload(): void {
    console.log('[LangChain RAG] Extension unloading')
    if (this.serviceStarted) {
      this.shutdownService().catch(err => {
        console.error('[LangChain RAG] Error shutting down service:', err)
      })
    }
  }

  /**
   * Listen for workspace files added and ingest them to LangChain RAG
   */
  private setupWorkspaceFileListener(): void {
    console.log('[LangChain RAG] 📡 Setting up workspace file listener...')
    
    events.on(WorkspaceEvent.OnFileAdded, async (file: WorkspaceFile) => {
      console.log('[LangChain RAG] 📥 WorkspaceEvent.OnFileAdded triggered:', {
        file_id: file.id,
        file_name: file.name,
        file_path: file.file_path,
        workspace_id: file.workspace_id,
        enabled: this.config.enabled
      })
      
      if (!this.config.enabled) {
        console.log('[LangChain RAG] Extension disabled, skipping file indexing')
        return
      }

      // Skip if already ingesting this file
      if (this.ingestionInProgress.has(file.id)) {
        console.log('[LangChain RAG] Ingestion already in progress for file:', file.id)
        return
      }

      console.log('[LangChain RAG] 📂 Workspace file added, starting indexing:', file.id, file.name)
      console.log('[LangChain RAG] 🗂️  Collection: enterprise_knowledge (global)')
      console.log('[LangChain RAG] 🏢 Workspace ID:', file.workspace_id)

      // Mark as in-progress
      this.ingestionInProgress.add(file.id)

      try {
        // Ingest the file to LangChain RAG
        await this.ingestWorkspaceFile(file.workspace_id, file.id, file.file_path).catch((err) => {
          console.error('[LangChain RAG] Error in async workspace file ingestion:', err)
        }).finally(() => {
          // Mark as complete
          this.ingestionInProgress.delete(file.id)
        })
      } catch (err) {
        console.error('[LangChain RAG] Error setting up workspace file ingestion:', err)
        this.ingestionInProgress.delete(file.id)
      }
    })
    
    console.log('[LangChain RAG] ✓ Workspace file listener registered')
  }

  /**
   * Ingest a workspace file to LangChain RAG
   */
  private async ingestWorkspaceFile(
    workspaceId: string,
    fileId: string,
    filePath: string
  ): Promise<void> {
    try {
      const collection = this.getCollectionName()

      console.log(`[LangChain RAG] Ingesting workspace file ${fileId} to global collection ${collection}`)
      console.log(`[LangChain RAG] Workspace ID in metadata: ${workspaceId}`)

      // Fetch workspace info to get the name
      let workspaceName = 'Unknown'
      try {
        const workspace = await invoke<any>('get_workspace', { workspaceId })
        workspaceName = workspace?.name || workspaceId
        console.log(`[LangChain RAG] Workspace name: ${workspaceName}`)
      } catch (err) {
        console.warn(`[LangChain RAG] Failed to fetch workspace name:`, err)
      }

      // Ingest to LangChain with workspace_id and workspace_name in metadata for filtering
      const ingestParams: IngestParams = {
        file_path: filePath,
        collection: collection,
        chunk_size: this.config.chunk_size,
        chunk_overlap: this.config.chunk_overlap,
        metadata: {
          workspace_id: workspaceId,
          workspace_name: workspaceName,
          file_id: fileId,
        },
      }

      const response = await invoke<IngestResponse>('plugin:langchain|ingest_document', { params: ingestParams })

      console.log(`[LangChain RAG] File ${fileId} ingested successfully:`, response)
    } catch (err) {
      console.error(`[LangChain RAG] Failed to ingest file ${fileId}:`, err)
      throw err
    }
  }

  /**
   * Start the LangChain Python service
   */
  private async ensureServiceRunning(): Promise<void> {
    if (this.serviceStarted) {
      return
    }

    try {
      console.log('[LangChain RAG] Starting Python service...')
      
      const serviceConfig: ServiceConfig = {
        call_timeout_ms: 60000,  // 60 seconds for LLM generation
        auto_restart: true,
        max_restart_attempts: 3,
        restart_count: 0,
        restart_delay_ms: 1000,  // 1 second delay between restarts
        health_check_interval_ms: 30000,  // 30 seconds health check interval
      }

      await invoke('plugin:langchain|spawn_langchain_service', {
        binaryPath: null,
        envVars: null,
        config: serviceConfig,
      })

      // Initialize with config
      const initConfig = {
        embeddings_model: this.config.embeddings_model,
        chunk_size: this.config.chunk_size,
        chunk_overlap: this.config.chunk_overlap,
        top_k: this.config.retrieval_limit,
        llm_server_url: this.config.llm_server_url,
        llm_temperature: this.config.llm_temperature,
        llm_max_tokens: this.config.llm_max_tokens,
      }

      // Note: Initialization happens automatically in the service
      this.serviceStarted = true
      console.log('[LangChain RAG] Service started successfully')
      
    } catch (error) {
      console.error('[LangChain RAG] Failed to start service:', error)
      throw error
    }
  }

  /**
   * Shutdown the LangChain service
   */
  private async shutdownService(): Promise<void> {
    try {
      await invoke('plugin:langchain|shutdown_langchain_service')
      this.serviceStarted = false
      console.log('[LangChain RAG] Service shut down')
    } catch (error) {
      console.error('[LangChain RAG] Error shutting down service:', error)
    }
  }

  /**
   * Get the global collection name
   */
  private getCollectionName(): string {
    return 'enterprise_knowledge'
  }

  /**
   * Retrieve relevant documents with threshold filtering (for pre-retrieval)
   * Returns only documents above the relevance threshold
   * Supports workspace-scoped search via workspace_id filter
   */
  async retrieveDocuments(query: string, threshold: number = 0.5, workspaceId?: string): Promise<{
    sources: Array<{ content: string; metadata: Record<string, unknown>; score: number }>
    num_sources: number
  }> {
    if (!this.config.enabled) {
      console.log('[LangChain RAG] Extension disabled, returning empty sources')
      return { sources: [], num_sources: 0 }
    }

    await this.ensureServiceRunning()
    const collection = this.getCollectionName()

    console.log('[LangChain RAG] ====== PRE-RETRIEVAL MODE ======')
    console.log('[LangChain RAG] Workspace ID:', workspaceId || 'none (global search)')
    console.log('[LangChain RAG] Collection:', collection)
    console.log('[LangChain RAG] Query:', query)
    console.log('[LangChain RAG] Threshold:', threshold)

    try {
      // Build filter for workspace-scoped search
      const filter = workspaceId ? { workspace_id: workspaceId } : undefined
      
      if (filter) {
        console.log('[LangChain RAG] Applying filter:', JSON.stringify(filter))
      } else {
        console.log('[LangChain RAG] No filter - searching all documents')
      }

      const params: QueryParams = {
        query,
        collection,
        k: this.config.retrieval_limit,
        filter,
      }

      const result = await invoke<QueryResponse>('plugin:langchain|query_rag', { params })

      console.log('[LangChain RAG] Query response received')
      console.log('[LangChain RAG]   Query:', result.query)
      console.log('[LangChain RAG]   Collection:', result.collection)
      console.log('[LangChain RAG]   Retrieved:', result.sources?.length || 0, 'documents')
      console.log('[LangChain RAG]   Answer length:', result.answer?.length || 0, 'chars')

      if (!result.sources || !Array.isArray(result.sources)) {
        console.warn('[LangChain RAG] No sources in response')
        return { sources: [], num_sources: 0 }
      }

      // Filter by threshold
      const filtered = result.sources.filter(s => s.score >= threshold)
      
      console.log('[LangChain RAG] Filtered to', filtered.length, 'documents above threshold', threshold)
      filtered.forEach((source, idx) => {
        console.log(`[LangChain RAG]   ${idx + 1}. Score: ${(source.score * 100).toFixed(1)}% - ${source.metadata?.source_file || 'unknown'}`)
      })

      return { sources: filtered, num_sources: filtered.length }
    } catch (error) {
      console.error('[LangChain RAG] Pre-retrieval failed:', error)
      // Log full error details for debugging
      if (error && typeof error === 'object') {
        console.error('[LangChain RAG] Error details:', JSON.stringify(error, null, 2))
      }
      return { sources: [], num_sources: 0 }
    }
  }

  /**
   * Query the RAG system (returns a tool result for MCP compatibility)
   */
  private async queryRag(query: string, fileIds?: string[], workspaceId?: string): Promise<QueryResponse> {
    await this.ensureServiceRunning()

    const collection = this.getCollectionName()

    console.log('[LangChain RAG] Starting query...')
    console.log('[LangChain RAG] Workspace ID:', workspaceId || 'none (global search)')
    console.log('[LangChain RAG] Collection name:', collection)
    console.log('[LangChain RAG] Query text:', query)
    console.log('[LangChain RAG] Retrieval limit (k):', this.config.retrieval_limit)
    console.log('[LangChain RAG] File IDs filter:', fileIds || 'none')

    // Build filter combining workspace_id and file_ids if provided
    let filter: Record<string, unknown> | undefined
    if (workspaceId && fileIds) {
      filter = { workspace_id: workspaceId, file_id: fileIds }
    } else if (workspaceId) {
      filter = { workspace_id: workspaceId }
    } else if (fileIds) {
      filter = { file_id: fileIds }
    }

    const params: QueryParams = {
      query,
      collection,
      k: this.config.retrieval_limit,
      filter,
    }

    console.log('[LangChain RAG] Sending query params to Python service...')
    
    const result = await invoke<QueryResponse>('plugin:langchain|query_rag', {
      params,
    })

    console.log('[LangChain RAG] Query response received')
    console.log('[LangChain RAG] Answer length:', result.answer?.length || 0, 'chars')
    console.log('[LangChain RAG] Total sources returned:', result.num_sources)
    
    // Log detailed retrieval information
    if (result.sources && result.sources.length > 0) {
      console.log('[LangChain RAG] ====== RETRIEVED DOCUMENTS ======')
      result.sources.forEach((source, index) => {
        console.log(`[LangChain RAG] Document ${index + 1}:`)
        console.log(`  Relevance Score: ${(source.score * 100).toFixed(2)}%`)
        console.log(`  Source File: ${source.metadata?.source_file || 'unknown'}`)
        console.log(`  Chunk Index: ${source.metadata?.chunk_index || 'N/A'}`)
        console.log(`  Content Preview: ${source.content?.substring(0, 150).replace(/\n/g, ' ')}...`)
      })
      console.log('[LangChain RAG] ====== END DOCUMENTS ======')
    } else {
      console.log('[LangChain RAG] No documents retrieved')
    }

    return result
  }

  async getTools(): Promise<MCPTool[]> {
    console.log('[LangChain RAG] ========== getTools() CALLED ==========')
    console.log('[LangChain RAG] Extension type:', this.type())
    console.log('[LangChain RAG] Extension enabled:', this.config.enabled)
    
    const tool: MCPTool = {
      name: 'retrieve_langchain',
      description: 'ALWAYS search the workspace documents first before answering any question. This tool retrieves relevant context from ingested workspace documents and knowledge base using LangChain vector search. Use this for ANY query that could benefit from workspace-specific information, documentation, or context. Returns an AI-generated answer with source citations.',
      inputSchema: {
        type: 'object',
        properties: {
          workspace_id: {
            type: 'string',
            description: 'Optional workspace ID to scope the search to a specific workspace',
          },
          query: {
            type: 'string',
            description: 'The search query or question',
          },
          file_ids: {
            type: 'array',
            items: { type: 'string' },
            description: 'Optional: Filter by specific file IDs',
          },
        },
        required: ['query'],
      },
      server: 'langchain-rag',
    }
    
    console.log('[LangChain RAG] Returning tool:', tool.name)
    return [tool]
  }

  async getToolNames(): Promise<string[]> {
    return ['retrieve_langchain']
  }

  async callTool(toolName: string, args: Record<string, unknown>): Promise<MCPToolCallResult> {
    console.log('[LangChain RAG] ========== callTool() ENTRY POINT ==========')
    console.log('[LangChain RAG] Tool name received:', toolName)
    console.log('[LangChain RAG] Args received:', JSON.stringify(args, null, 2))
    
    if (!this.config.enabled) {
      console.log('[LangChain RAG] Extension is DISABLED - returning error')
      return {
        error: 'LangChain RAG is disabled in settings.',
        content: [{ type: 'text', text: 'LangChain RAG is disabled in settings.' }],
      }
    }

    console.log('[LangChain RAG] Extension is ENABLED - proceeding')
    console.log('[LangChain RAG] Checking if toolName === "retrieve_langchain":', toolName === 'retrieve_langchain')

    if (toolName === 'retrieve_langchain') {
      console.log('[LangChain RAG] ✓ Tool name matches - proceeding with retrieve_langchain')
      const workspaceId = args.workspace_id as string
      const query = args.query as string
      const fileIds = args.file_ids as string[] | undefined

      console.log('[LangChain RAG] Extracted args:')
      console.log('[LangChain RAG]   workspaceId:', workspaceId || 'none (global)')
      console.log('[LangChain RAG]   query:', query)
      console.log('[LangChain RAG]   fileIds:', fileIds)

      if (!query) {
        console.error('[LangChain RAG] Missing required parameter: query')
        return {
          error: 'Missing required parameter: query',
          content: [{ type: 'text', text: 'Missing required parameter: query' }],
        }
      }

      console.log('[LangChain RAG]   🗂️  Collection: enterprise_knowledge')
      console.log('[LangChain RAG]   🔍  Workspace filter:', workspaceId || 'none (global)')

      try {
        console.log('[LangChain RAG] Calling queryRag...')
        const result = await this.queryRag(query, fileIds, workspaceId)

        console.log('[LangChain RAG] queryRag returned successfully')
        console.log('[LangChain RAG] Result has', result.sources.length, 'sources')
        console.log('[LangChain RAG] Result object keys:', Object.keys(result))
        
        // Verify sources exist
        if (!result.sources || !Array.isArray(result.sources)) {
          console.error('[LangChain RAG] ERROR: sources is not an array!', result.sources)
          throw new Error('Invalid sources format from queryRag')
        }
        
        // Log ALL retrieved documents with scores BEFORE filtering
        console.log('[LangChain RAG] ====== ALL RETRIEVED DOCUMENTS ======')
        result.sources.forEach((source, idx) => {
          console.log(`[LangChain RAG] 📄 Document ${idx + 1}/${result.sources.length}:`)
          console.log(`  Score: ${(source.score * 100).toFixed(2)}% (${source.score.toFixed(4)})`)
          console.log(`  File: ${source.metadata?.source_file || 'unknown'}`)
          console.log(`  Chunk: ${source.metadata?.chunk_index || 'N/A'}`)
          console.log(`  Content Preview: ${source.content?.substring(0, 150).replace(/\n/g, ' ')}...`)
        })
        console.log('[LangChain RAG] ====== END ALL DOCUMENTS ======')
        
        // Filter sources by relevance threshold (default 0.5)
        const threshold = 0.5
        console.log('[LangChain RAG] Applying threshold filter:', threshold)
        console.log('[LangChain RAG] ====== THRESHOLD FILTERING ======')
        console.log('[LangChain RAG] Total retrieved sources:', result.sources.length)
        
        const relevantSources = result.sources.filter((source, filterIndex) => {
          const isRelevant = source.score >= threshold
          const passFailIcon = isRelevant ? '✓ PASS' : '✗ FAIL'
          console.log(`[LangChain RAG] Source ${filterIndex + 1}: Score ${(source.score * 100).toFixed(2)}% - ${passFailIcon}`)
          if (!isRelevant) {
            console.log(`  File: ${source.metadata?.source_file || 'unknown'}`)
            console.log(`  Content: ${source.content?.substring(0, 100).replace(/\n/g, ' ')}...`)
          }
          return isRelevant
        })

        console.log('[LangChain RAG] ====== FILTERING RESULTS ======')
        console.log('[LangChain RAG] Relevant sources after threshold:', relevantSources.length, `/ ${result.sources.length}`)
        console.log('[LangChain RAG] Filter calculation complete')
        
        if (relevantSources.length > 0) {
          console.log('[LangChain RAG] ====== ACCEPTED DOCUMENTS ======')
          relevantSources.forEach((source, idx) => {
            console.log(`[LangChain RAG] ✓ Document ${idx + 1}:`)
            console.log(`  Relevance Score: ${(source.score * 100).toFixed(2)}%`)
            console.log(`  Source File: ${source.metadata?.source_file || 'unknown'}`)
            console.log(`  Chunk Index: ${source.metadata?.chunk_index || 'N/A'}`)
            console.log(`  Content: ${source.content?.substring(0, 120).replace(/\n/g, ' ')}...`)
          })
          console.log('[LangChain RAG] ====== END ACCEPTED ======')
        } else {
          console.log('[LangChain RAG] No sources passed the threshold filter')
        }

        if (relevantSources.length === 0) {
          console.log('[LangChain RAG] No sources above threshold (0.5), returning empty context')
          console.log('[LangChain RAG] Original query had', result.num_sources, 'total sources')
          console.log('[LangChain RAG] All sources were filtered out - threshold was too high or scores too low')
          return {
            error: '',
            content: [{ 
              type: 'text', 
              text: `No relevant documents found with confidence above threshold (0.5). Original query had ${result.num_sources} matches but all were below threshold.` 
            }],
          }
        }

        // Format response
        let response = `**Answer:**\n${result.answer}\n\n`

        if (relevantSources.length > 0) {
          response += `**Sources (${relevantSources.length} above 0.5 threshold):**\n`
          relevantSources.forEach((source, idx) => {
            response += `\n${idx + 1}. [Score: ${source.score.toFixed(3)}]\n`
            response += `${source.content.substring(0, 200)}...\n`
          })
        }

        console.log('[LangChain RAG] Formatted response:', response.substring(0, 200))
        console.log('[LangChain RAG] Tool call complete - returning formatted response')
        
        return {
          error: '',
          content: [{ type: 'text', text: response }],
        }
      } catch (error) {
        console.error('[LangChain RAG] Query failed:', error)
        return {
          error: String(error),
          content: [{ type: 'text', text: `Query failed: ${error}` }],
        }
      }
    }

    console.error('[LangChain RAG] ✗ Tool name NOT matched!')
    console.log('[LangChain RAG] Expected: "retrieve_langchain"')
    console.log('[LangChain RAG] Received:', toolName)
    console.log('[LangChain RAG] Type of received:', typeof toolName)
    console.log('[LangChain RAG] Character codes:', toolName.split('').map(c => c.charCodeAt(0)))
    
    return {
      error: `Unknown tool: ${toolName}`,
      content: [{ type: 'text', text: `Unknown tool: ${toolName}` }],
    }
  }

  /**
   * Get registered settings definitions
   */
  async getSettings(): Promise<SettingComponentProps[]> {
    try {
      return structuredClone(SETTINGS) as SettingComponentProps[]
    } catch (error) {
      console.error('[LangChain RAG] Error getting settings:', error)
      return []
    }
  }

  onSettingUpdate<T>(key: string, value: T): void {
    console.log('[LangChain RAG] Setting updated:', key, value)
    
    // Update config
    if (key in this.config) {
      (this.config as any)[key] = value
    }

    // Handle enable/disable
    if (key === 'enabled') {
      if (value) {
        this.ensureServiceRunning().catch(err => {
          console.error('[LangChain RAG] Failed to start service:', err)
        })
      } else {
        this.shutdownService().catch(err => {
          console.error('[LangChain RAG] Failed to shutdown service:', err)
        })
      }
    }
  }
}

// Settings definitions
const SETTINGS: SettingComponentProps[] = [
  {
    key: 'enabled',
    title: 'Enable LangChain RAG',
    description: 'Enable LangChain-powered RAG for document retrieval',
    controllerType: 'checkbox',
    controllerProps: {
      value: true,  // Enabled by default
    },
  },
  {
    key: 'llm_server_url',
    title: 'LLM Server URL',
    description: 'URL of the LLM server (e.g., http://127.0.0.1:8080/v1)',
    controllerType: 'input',
    controllerProps: {
      value: 'http://127.0.0.1:8080/v1',
      placeholder: 'http://127.0.0.1:8080/v1',
    },
  },
  {
    key: 'embeddings_model',
    title: 'Embeddings Model',
    description: 'Model name for embeddings (e.g., sentence-transformers/all-MiniLM-L6-v2)',
    controllerType: 'input',
    controllerProps: {
      value: 'sentence-transformers/all-MiniLM-L6-v2',
      placeholder: 'sentence-transformers/all-MiniLM-L6-v2',
    },
  },
  {
    key: 'retrieval_limit',
    title: 'Retrieval Limit',
    description: 'Maximum number of documents to retrieve per query',
    controllerType: 'slider',
    controllerProps: {
      value: 3,
      min: 1,
      max: 20,
      step: 1,
    },
  },
  {
    key: 'chunk_size',
    title: 'Chunk Size (tokens)',
    description: 'Size of chunks when splitting documents',
    controllerType: 'slider',
    controllerProps: {
      value: 512,
      min: 128,
      max: 2048,
      step: 128,
    },
  },
  {
    key: 'chunk_overlap',
    title: 'Chunk Overlap (tokens)',
    description: 'Overlap between chunks',
    controllerType: 'slider',
    controllerProps: {
      value: 64,
      min: 0,
      max: 512,
      step: 32,
    },
  },
  {
    key: 'llm_temperature',
    title: 'LLM Temperature',
    description: 'Temperature for LLM generation (0-1)',
    controllerType: 'slider',
    controllerProps: {
      value: 0.7,
      min: 0,
      max: 1,
      step: 0.1,
    },
  },
  {
    key: 'llm_max_tokens',
    title: 'LLM Max Tokens',
    description: 'Maximum tokens to generate',
    controllerType: 'slider',
    controllerProps: {
      value: 1024,
      min: 256,
      max: 4096,
      step: 256,
    },
  },
]
