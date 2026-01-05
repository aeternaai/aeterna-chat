/**
 * TypeScript bindings for the LangChain plugin
 * 
 * Provides type-safe access to LangChain RAG capabilities via Tauri commands.
 */

import { invoke } from '@tauri-apps/api/core'

// ============================================================================
// Types
// ============================================================================

/**
 * Service configuration for the LangChain Python subprocess
 */
export interface ServiceConfig {
  /** Timeout for IPC calls in milliseconds */
  call_timeout_ms?: number
  /** Whether to auto-restart on crash */
  auto_restart?: boolean
  /** Maximum restart attempts */
  max_restart_attempts?: number
  /** Delay between restart attempts in milliseconds */
  restart_delay_ms?: number
  /** Health check interval in milliseconds */
  health_check_interval_ms?: number
}

/**
 * Information about the running service
 */
export interface ServiceInfo {
  pid: number
  binary_path: string
  started_at: number
  is_healthy: boolean
  last_health_check?: number
  request_count: number
  error_count: number
}

/**
 * Health check response
 */
export interface HealthResponse {
  status: string
  version: string
  uptime_seconds: number
  initialized?: boolean
  langchain_available?: boolean
}

/**
 * Parameters for document ingestion
 */
export interface IngestDocumentParams {
  /** Path to the document file */
  file_path: string
  /** Collection name for storage */
  collection: string
  /** Chunk size in characters (optional) */
  chunk_size?: number
  /** Chunk overlap in characters (optional) */
  chunk_overlap?: number
  /** Additional metadata to attach to chunks */
  metadata?: Record<string, unknown>
}

/**
 * Result of document ingestion
 */
export interface IngestResponse {
  collection: string
  chunks_count: number
  document_ids: string[]
  source_file: string
}

/**
 * Parameters for RAG query
 */
export interface QueryRagParams {
  /** The question to answer */
  query: string
  /** Collection to search */
  collection: string
  /** Number of documents to retrieve */
  k?: number
  /** Minimum similarity score threshold */
  score_threshold?: number
  /** Optional metadata filter */
  filter?: Record<string, unknown>
}

/**
 * Source document from retrieval
 */
export interface SourceDocument {
  content: string
  metadata: Record<string, unknown>
  score: number
}

/**
 * Result of RAG query
 */
export interface QueryResponse {
  answer: string
  sources: SourceDocument[]
  num_sources: number
  query: string
  collection: string
}

/**
 * Collection information
 */
export interface CollectionInfo {
  name: string
  vectors_count: number
  points_count: number
}

/**
 * Pipeline configuration and status
 */
export interface PipelineInfo {
  version: string
  initialized: boolean
  config: Record<string, unknown>
  collections: CollectionInfo[]
  uptime_seconds: number
}

/**
 * Extended service information
 */
export interface ServiceInfoExtended {
  is_running: boolean
  service?: ServiceInfo
  pipeline?: PipelineInfo
}

// ============================================================================
// API Functions
// ============================================================================

/**
 * Start the LangChain Python service
 * 
 * @param binaryPath - Optional custom path to Python binary
 * @param envVars - Optional environment variables
 * @param config - Optional service configuration
 * @returns Service information
 */
export async function spawnLangchainService(
  binaryPath?: string,
  envVars?: Record<string, string>,
  config?: ServiceConfig
): Promise<ServiceInfo> {
  return await invoke('plugin:langchain|spawn_langchain_service', {
    binaryPath,
    envVars,
    config,
  })
}

/**
 * Stop the LangChain Python service
 */
export async function shutdownLangchainService(): Promise<void> {
  return await invoke('plugin:langchain|shutdown_langchain_service')
}

/**
 * Check the health of the LangChain service
 * 
 * @returns Health status information
 */
export async function healthCheck(): Promise<HealthResponse> {
  return await invoke('plugin:langchain|health_check')
}

/**
 * Ingest a document into the RAG system
 * 
 * Processes the document, generates embeddings, and stores in the specified collection.
 * 
 * @param params - Ingestion parameters
 * @returns Ingestion results with chunk count and document IDs
 */
export async function ingestDocument(params: IngestDocumentParams): Promise<IngestResponse> {
  return await invoke('plugin:langchain|ingest_document', { params })
}

/**
 * Query the RAG system
 * 
 * Retrieves relevant context and returns sources.
 * 
 * @param params - Query parameters
 * @returns Query results with answer and source documents
 */
export async function queryRag(params: QueryRagParams): Promise<QueryResponse> {
  return await invoke('plugin:langchain|query_rag', { params })
}

/**
 * Get information about the LangChain service
 * 
 * Returns configuration, status, and pipeline information.
 * 
 * @returns Extended service information
 */
export async function getServiceInfo(): Promise<ServiceInfoExtended> {
  return await invoke('plugin:langchain|get_service_info')
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Check if the service is running
 * 
 * @returns true if the service is running
 */
export async function isServiceRunning(): Promise<boolean> {
  try {
    const info = await getServiceInfo()
    return info.is_running
  } catch {
    return false
  }
}

/**
 * Ensure the service is running, starting it if necessary
 * 
 * @param config - Optional service configuration
 * @returns Service information
 */
export async function ensureServiceRunning(config?: ServiceConfig): Promise<ServiceInfo> {
  const info = await getServiceInfo()
  if (info.is_running && info.service) {
    return info.service
  }
  return await spawnLangchainService(undefined, undefined, config)
}

/**
 * Ingest multiple documents in sequence
 * 
 * @param documents - Array of ingestion parameters
 * @returns Array of ingestion results
 */
export async function ingestDocuments(documents: IngestDocumentParams[]): Promise<IngestResponse[]> {
  const results: IngestResponse[] = []
  for (const params of documents) {
    const result = await ingestDocument(params)
    results.push(result)
  }
  return results
}
