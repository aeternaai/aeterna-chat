import { RAGExtension, MCPTool, MCPToolCallResult, ExtensionTypeEnum, VectorDBExtension, type AttachmentInput, type SettingComponentProps, AIEngine, type AttachmentFileInfo } from '@janhq/core'
import './env.d'
import { getRAGTools, RETRIEVE, LIST_ATTACHMENTS, GET_CHUNKS } from './tools'
import { events } from '@janhq/core'
import { WorkspaceEvent, type WorkspaceFile } from '@janhq/core'
import { invoke } from '@tauri-apps/api/core'

export default class RagExtension extends RAGExtension {
  private config = {
    enabled: true,
    retrievalLimit: 3,
    retrievalThreshold: 0.3,
    chunkSizeTokens: 512,
    overlapTokens: 64,
    searchMode: 'auto' as 'auto' | 'ann' | 'linear',
    maxFileSizeMB: 20,
  }
  private workspaceFileListenerSetup = false
  private ingestionInProgress = new Set<string>() // Track in-progress ingestions by fileId

  async onLoad(): Promise<void> {
    const settings = structuredClone(SETTINGS) as SettingComponentProps[]
    await this.registerSettings(settings)
    this.config.enabled = await this.getSetting('enabled', this.config.enabled)
    this.config.maxFileSizeMB = await this.getSetting('max_file_size_mb', this.config.maxFileSizeMB)
    this.config.retrievalLimit = await this.getSetting('retrieval_limit', this.config.retrievalLimit)
    this.config.retrievalThreshold = await this.getSetting('retrieval_threshold', this.config.retrievalThreshold)
    this.config.chunkSizeTokens = await this.getSetting('chunk_size_tokens', this.config.chunkSizeTokens)
    this.config.overlapTokens = await this.getSetting('overlap_tokens', this.config.overlapTokens)
    this.config.searchMode = await this.getSetting('search_mode', this.config.searchMode)

    // Check ANN availability on load
    try {
      const vec = window.core?.extensionManager.get(ExtensionTypeEnum.VectorDB) as unknown as VectorDBExtension
      if (vec?.getStatus) {
        const status = await vec.getStatus()
        console.log('[RAG] Vector DB ANN support:', status.ann_available ? '✓ AVAILABLE' : '✗ NOT AVAILABLE')
        if (!status.ann_available) {
          console.warn('[RAG] Warning: sqlite-vec not loaded. Collections will use slower linear search.')
        }
      }
    } catch (e) {
      console.error('[RAG] Failed to check ANN status:', e)
    }

    // Setup workspace file listener (only once)
    if (!this.workspaceFileListenerSetup) {
      this.setupWorkspaceFileListener()
      this.workspaceFileListenerSetup = true
    }
  }

  private setupWorkspaceFileListener(): void {
    // Listen for new workspace files added
    events.on(WorkspaceEvent.OnFileAdded, async (file: WorkspaceFile) => {
      if (!this.config.enabled) {
        console.log('[RAG] RAG disabled, skipping file indexing')
        return
      }

      // Skip if already ingesting this file
      if (this.ingestionInProgress.has(file.id)) {
        console.log('[RAG] Ingestion already in progress for file:', file.id)
        return
      }

      console.log('[RAG] Workspace file added, starting indexing:', file.id, file.name)

      // Mark as in-progress
      this.ingestionInProgress.add(file.id)

      try {
        // Ingest the file directly in the extension
        // This will handle parsing, chunking, embedding, and storage
        this.ingestWorkspaceFile(file.workspace_id, file.id, file.file_path).catch((err) => {
          console.error('[RAG] Error in async workspace file ingestion:', err)
        }).finally(() => {
          // Mark as complete
          this.ingestionInProgress.delete(file.id)
        })
      } catch (err) {
        console.error('[RAG] Failed to start workspace file indexing:', err)
        this.ingestionInProgress.delete(file.id)
      }
    })

    // Listen for workspace file deletions - clean up vector data if needed
    events.on(WorkspaceEvent.OnFileRemoved, async (data: any) => {
      console.log('[RAG] Workspace file removed, cleanup requested for:', data.fileId)
      // Cleanup logic would go here if needed
    })

    console.log('[RAG] Workspace file listener setup complete')
  }

  onUnload(): void {}

  async getTools(): Promise<MCPTool[]> {
    return getRAGTools(this.config.retrievalLimit)
  }

  async getToolNames(): Promise<string[]> {
    // Keep this in sync with getTools() but without building full schemas
    return [LIST_ATTACHMENTS, RETRIEVE, GET_CHUNKS]
  }

  async callTool(toolName: string, args: Record<string, unknown>): Promise<MCPToolCallResult> {
    switch (toolName) {
      case LIST_ATTACHMENTS:
        return this.listAttachments(args)
      case RETRIEVE:
        return this.retrieve(args)
      case GET_CHUNKS:
        return this.getChunks(args)
      default:
        return {
          error: `Unknown tool: ${toolName}`,
          content: [{ type: 'text', text: `Unknown tool: ${toolName}` }],
        }
    }
  }

  private async listAttachments(args: Record<string, unknown>): Promise<MCPToolCallResult> {
    const threadId = String(args['thread_id'] || '')
    if (!threadId) {
      return { error: 'Missing thread_id', content: [{ type: 'text', text: 'Missing thread_id' }] }
    }
    try {
      const vec = window.core?.extensionManager.get(ExtensionTypeEnum.VectorDB) as unknown as VectorDBExtension
      if (!vec?.listAttachments) {
        return { error: 'Vector DB extension missing listAttachments', content: [{ type: 'text', text: 'Vector DB extension missing listAttachments' }] }
      }
      const files = await vec.listAttachments(threadId)
      return {
        error: '',
        content: [
          {
            type: 'text',
            text: JSON.stringify({ thread_id: threadId, attachments: files || [] }),
          },
        ],
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      return { error: msg, content: [{ type: 'text', text: `List attachments failed: ${msg}` }] }
    }
  }

  private async retrieve(args: Record<string, unknown>): Promise<MCPToolCallResult> {
    const threadId = String(args['thread_id'] || '')
    const query = String(args['query'] || '')
    const fileIds = args['file_ids'] as string[] | undefined

    const s = this.config
    const topK = (args['top_k'] as number) || s.retrievalLimit || 3
    const threshold = s.retrievalThreshold ?? 0.3
    const mode: 'auto' | 'ann' | 'linear' = s.searchMode || 'auto'

    if (s.enabled === false) {
      return {
        error: 'Attachments feature disabled',
        content: [
          {
            type: 'text',
            text: 'Attachments are disabled in Settings. Enable them to use retrieval.',
          },
        ],
      }
    }
    if (!threadId || !query) {
      return {
        error: 'Missing thread_id or query',
        content: [{ type: 'text', text: 'Missing required parameters' }],
      }
    }

    try {
      // Resolve extensions
      const vec = window.core?.extensionManager.get(ExtensionTypeEnum.VectorDB) as unknown as VectorDBExtension
      if (!vec?.searchCollection) {
        return {
          error: 'RAG dependencies not available',
          content: [
            { type: 'text', text: 'Vector DB extension not available' },
          ],
        }
      }

      const queryEmb = (await this.embedTexts([query]))?.[0]
      if (!queryEmb) {
        return {
          error: 'Failed to compute embeddings',
          content: [{ type: 'text', text: 'Failed to compute embeddings' }],
        }
      }

      // Get results from thread collection
      const threadResults = await vec.searchCollection(
        threadId,
        queryEmb,
        topK,
        threshold,
        mode,
        fileIds
      )

      // Try to also get results from workspace collection if thread belongs to a workspace
      let workspaceResults: any[] = []
      try {
        const thread = await window.core?.api?.getThread?.(threadId)
        if (thread?.workspace_id) {
          const workspaceCollectionName = `workspace_${thread.workspace_id}`
          workspaceResults = await invoke<any[]>('plugin:vector-db|search_collection', {
            collection: workspaceCollectionName,
            queryEmbedding: queryEmb,
            limit: topK,
            threshold,
            mode,
            fileIds,
          }).catch(() => [])
        }
      } catch (err) {
        console.log('[RAG] Workspace collection search skipped:', err)
        // Silently skip workspace search if it fails
      }

      // Merge results: thread collection takes precedence, then workspace
      const allResults = [
        ...threadResults,
        ...workspaceResults.filter((wr) => !threadResults.some((tr) => tr.id === wr.id)),
      ].slice(0, topK)

      const payload = {
        thread_id: threadId,
        query,
        citations: allResults?.map((r: any) => ({
          id: r.id,
          text: r.text,
          score: r.score,
          file_id: r.file_id,
          chunk_file_order: r.chunk_file_order
        })) ?? [],
        mode,
      }
      return { error: '', content: [{ type: 'text', text: JSON.stringify(payload) }] }
    } catch (e) {
      console.error('[RAG] Retrieve error:', e)
      let msg = 'Unknown error'
      if (e instanceof Error) {
        msg = e.message
      } else if (typeof e === 'string') {
        msg = e
      } else if (e && typeof e === 'object') {
        msg = JSON.stringify(e)
      }
      return { error: msg, content: [{ type: 'text', text: `Retrieve failed: ${msg}` }] }
    }
  }

  private async getChunks(args: Record<string, unknown>): Promise<MCPToolCallResult> {
    const threadId = String(args['thread_id'] || '')
    const fileId = String(args['file_id'] || '')
    const startOrder = args['start_order'] as number | undefined
    const endOrder = args['end_order'] as number | undefined

    if (!threadId || !fileId || startOrder === undefined || endOrder === undefined) {
      return {
        error: 'Missing thread_id, file_id, start_order, or end_order',
        content: [{ type: 'text', text: 'Missing required parameters' }],
      }
    }

    try {
      const vec = window.core?.extensionManager.get(ExtensionTypeEnum.VectorDB) as unknown as VectorDBExtension
      if (!vec?.getChunks) {
        return {
          error: 'Vector DB extension not available',
          content: [{ type: 'text', text: 'Vector DB extension not available' }],
        }
      }

      const chunks = await vec.getChunks(threadId, fileId, startOrder, endOrder)

      const payload = {
        thread_id: threadId,
        file_id: fileId,
        chunks: chunks || [],
      }
      return { error: '', content: [{ type: 'text', text: JSON.stringify(payload) }] }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      return { error: msg, content: [{ type: 'text', text: `Get chunks failed: ${msg}` }] }
    }
  }

  // Desktop-only ingestion by file paths
  async ingestAttachments(
    threadId: string,
    files: AttachmentInput[]
  ): Promise<{ filesProcessed: number; chunksInserted: number; files: AttachmentFileInfo[] }> {
    if (!threadId || !Array.isArray(files) || files.length === 0) {
      return { filesProcessed: 0, chunksInserted: 0, files: [] }
    }

    // Respect feature flag: do nothing when disabled
    if (this.config.enabled === false) {
      return { filesProcessed: 0, chunksInserted: 0, files: [] }
    }

    const vec = window.core?.extensionManager.get(ExtensionTypeEnum.VectorDB) as unknown as VectorDBExtension
    if (!vec?.createCollection || !vec?.insertChunks) {
      throw new Error('Vector DB extension not available')
    }

    // Load settings
    const s = this.config
    const maxSize = (s?.enabled === false ? 0 : s?.maxFileSizeMB) || undefined
    const chunkSize = s?.chunkSizeTokens as number | undefined
    const chunkOverlap = s?.overlapTokens as number | undefined

    let totalChunks = 0
    const processedFiles: AttachmentFileInfo[] = []

    for (const f of files) {
      if (!f?.path) continue
      if (maxSize && f.size && f.size > maxSize * 1024 * 1024) {
        throw new Error(`File '${f.name}' exceeds size limit (${f.size} bytes > ${maxSize} MB).`)
      }

      const fileName = f.name || f.path.split(/[\\/]/).pop()
      // Preferred/required path: let Vector DB extension handle full file ingestion
      const canIngestFile = typeof (vec as any)?.ingestFile === 'function'
      if (!canIngestFile) {
        console.error('[RAG] Vector DB extension missing ingestFile; cannot ingest document')
        continue
      }
      const info = await (vec as VectorDBExtension).ingestFile(
        threadId,
        { path: f.path, name: fileName, type: f.type, size: f.size },
        { chunkSize: chunkSize ?? 512, chunkOverlap: chunkOverlap ?? 64 }
      )
      totalChunks += Number(info?.chunk_count || 0)
      processedFiles.push(info)
    }

    // Return files we ingested with real IDs directly from ingestFile
    return { filesProcessed: processedFiles.length, chunksInserted: totalChunks, files: processedFiles }
  }

  onSettingUpdate<T>(key: string, value: T): void {
    switch (key) {
      case 'enabled':
        this.config.enabled = Boolean(value)
        break
      case 'max_file_size_mb':
        this.config.maxFileSizeMB = Number(value)
        break
      case 'retrieval_limit':
        this.config.retrievalLimit = Number(value)
        break
      case 'retrieval_threshold':
        this.config.retrievalThreshold = Number(value)
        break
      case 'chunk_size_tokens':
        this.config.chunkSizeTokens = Number(value)
        break
      case 'overlap_tokens':
        this.config.overlapTokens = Number(value)
        break
      case 'search_mode':
        this.config.searchMode = String(value) as 'auto' | 'ann' | 'linear'
        break
    }
  }

  // Locally implement embedding logic (previously in embeddings-extension)
  private async embedTexts(texts: string[]): Promise<number[][]> {
    const llm = window.core?.extensionManager.getByName('@janhq/llamacpp-extension') as AIEngine & { embed?: (texts: string[]) => Promise<{ data: Array<{ embedding: number[]; index: number }> }> }
    if (!llm?.embed) throw new Error('llamacpp extension not available')
    const res = await llm.embed(texts)
    const data: Array<{ embedding: number[]; index: number }> = res?.data || []
    const out: number[][] = new Array(texts.length)
    for (const item of data) out[item.index] = item.embedding
    return out
  }

  /// Ingest a workspace file into the workspace collection
  async ingestWorkspaceFile(workspaceId: string, fileId: string, filePath: string): Promise<void> {
    try {
      console.log(`[RAG] Starting ingestion for file ${fileId} in workspace ${workspaceId}`)
      
      // Parse document
      console.log(`[RAG] Parsing document at: ${filePath}`)
      const text = await invoke<string>('plugin:rag|parse_document', {
        filePath,
        fileType: this.getMimeTypeFromPath(filePath),
      })

      if (!text || text.length === 0) {
        console.error(`[RAG] No text extracted from ${filePath}`)
        return
      }

      console.log(`[RAG] Parsed document, length: ${text.length}`)

      // Chunk text
      console.log(`[RAG] Chunking text with size=${this.config.chunkSizeTokens}, overlap=${this.config.overlapTokens}`)
      const chunks = await invoke<string[]>('plugin:vector-db|chunk_text', {
        text,
        chunkSize: this.config.chunkSizeTokens,
        chunkOverlap: this.config.overlapTokens,
      })

      if (!chunks || chunks.length === 0) {
        console.error(`[RAG] No chunks generated`)
        return
      }

      console.log(`[RAG] Created ${chunks.length} chunks`)

      // Get embeddings
      console.log(`[RAG] Generating embeddings for ${chunks.length} chunks`)
      const embeddings = await this.embedTexts(chunks)

      if (!embeddings || embeddings.length === 0 || embeddings[0].length === 0) {
        console.error(`[RAG] Failed to generate embeddings`)
        return
      }

      const dimension = embeddings[0].length
      console.log(`[RAG] Generated embeddings with dimension: ${dimension}`)

      // Create workspace collection
      const collectionName = `workspace_${workspaceId}`
      console.log(`[RAG] Creating collection: ${collectionName}`)
      await invoke('plugin:vector-db|create_collection', {
        name: collectionName,
        dimension,
      })

      // Create file entry
      console.log(`[RAG] Creating file entry in collection`)
      const fileInfo = await invoke('plugin:vector-db|create_file', {
        collection: collectionName,
        file: {
          path: filePath,
          name: filePath.split(/[\\/]/).pop(),
          type: this.getMimeTypeFromPath(filePath),
        },
      })

      // Insert chunks in batches with progress events
      const chunkInputs = chunks.map((text, i) => ({
        text,
        embedding: embeddings[i],
      }))

      const totalChunks = chunkInputs.length
      const batchSize = 10 // Insert in batches of 10 chunks
      console.log(`[RAG] Inserting ${totalChunks} chunks into collection in batches of ${batchSize}`)

      for (let i = 0; i < chunkInputs.length; i += batchSize) {
        const batch = chunkInputs.slice(i, Math.min(i + batchSize, chunkInputs.length))
        console.log(`[RAG] Inserting batch ${Math.floor(i / batchSize) + 1}/${Math.ceil(totalChunks / batchSize)}`)

        await invoke('plugin:vector-db|insert_chunks', {
          collection: collectionName,
          fileId: (fileInfo as any).id,
          chunks: batch,
        })

        // Emit progress every batch
        const processed = Math.min(i + batchSize, totalChunks)
        const percent = (processed / totalChunks) * 100
        window.core?.api?.emitEvent?.('workspace-file-indexing-progress', {
          file_id: fileId,
          percent,
          processed,
          total: totalChunks,
        })
      }

      // Update file status to indexed
      console.log(`[RAG] All chunks inserted, updating file status to indexed`)
      const now = Math.floor(Date.now() / 1000)
      await this.updateFileRagStatus(fileId, 'indexed', totalChunks, now, undefined)

      console.log(`[RAG] Successfully indexed workspace file ${fileId}: ${totalChunks} chunks`)
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err)
      console.error('[RAG] Ingest error details:', errorMsg, err)
      await this.updateFileRagStatus(fileId, 'failed', undefined, undefined, errorMsg)
    }
  }

  private async updateFileRagStatus(
    fileId: string,
    status: string,
    chunks?: number,
    indexedAt?: number,
    error?: string
  ): Promise<void> {
    try {
      await invoke('update_workspace_file_rag_status', {
        fileId,
        status,
        chunks,
        indexedAt,
        error,
      })
    } catch (err) {
      console.error('[RAG] Failed to update file RAG status:', err)
      throw err // Rethrow so caller can handle it properly
    }
  }

  private getMimeTypeFromPath(filePath: string): string {
    const ext = filePath.toLowerCase().split('.').pop() || ''
    const mimeMap: Record<string, string> = {
      'pdf': 'application/pdf',
      'txt': 'text/plain',
      'md': 'text/markdown',
      'docx': 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
      'doc': 'application/msword',
      'xlsx': 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
      'xls': 'application/vnd.ms-excel',
      'pptx': 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
      'ppt': 'application/vnd.ms-powerpoint',
      'html': 'text/html',
      'htm': 'text/html',
    }
    return mimeMap[ext] || 'application/octet-stream'
  }
}

export default RagExtension