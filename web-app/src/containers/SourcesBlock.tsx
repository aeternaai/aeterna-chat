import { ChevronDown, ChevronUp, BookOpen } from 'lucide-react'
import { useState, memo, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

interface SourceDocument {
  file: string
  chunk: string
  score: number
  chunk_index: number
  metadata?: {
    workspace_id?: string
    workspace_name?: string
    file_id?: string
    source_file?: string
  }
}

interface Props {
  sources: SourceDocument[]
  threshold: number
}

/**
 * SourcesBlock displays RAG retrieval sources in an expandable/collapsible block
 * Shows document names, workspace names, confidence scores, and chunk previews
 */
const SourcesBlock = memo(({ sources, threshold }: Props) => {
  const [isExpanded, setIsExpanded] = useState(false)
  const [workspaceNames, setWorkspaceNames] = useState<Record<string, string>>({})

  const handleClick = () => {
    setIsExpanded(!isExpanded)
  }

  // Fetch workspace names for sources that don't have them
  useEffect(() => {
    const fetchMissingWorkspaceNames = async () => {
      const names: Record<string, string> = {}
      
      for (const source of sources) {
        const workspaceId = source.metadata?.workspace_id
        const workspaceName = source.metadata?.workspace_name
        
        // If we have workspace_id but no workspace_name, fetch it
        if (workspaceId && !workspaceName && !workspaceNames[workspaceId]) {
          try {
            const workspace = await invoke<any>('get_workspace', { workspaceId })
            if (workspace?.name) {
              names[workspaceId] = workspace.name
            }
          } catch (err) {
            console.warn(`Failed to fetch workspace name for ${workspaceId}:`, err)
            names[workspaceId] = workspaceId // Fallback to ID
          }
        }
      }
      
      if (Object.keys(names).length > 0) {
        setWorkspaceNames(prev => ({ ...prev, ...names }))
      }
    }

    if (isExpanded && sources.length > 0) {
      fetchMissingWorkspaceNames()
    }
  }, [isExpanded, sources, workspaceNames])

  const getWorkspaceName = (source: SourceDocument): string => {
    const workspaceId = source.metadata?.workspace_id
    const workspaceName = source.metadata?.workspace_name
    
    if (workspaceName) return workspaceName
    if (workspaceId && workspaceNames[workspaceId]) return workspaceNames[workspaceId]
    if (workspaceId) return workspaceId
    return 'Unknown Workspace'
  }

  const getConfidenceBadge = (score: number) => {
    const isConfident = score >= 0.7
    const percentage = (score * 100).toFixed(0)
    
    return (
      <span
        className={`inline-flex items-center px-2 py-1 rounded text-xs font-medium ${
          isConfident
            ? 'bg-green-500/20 text-green-600 dark:text-green-400'
            : 'bg-orange-500/20 text-orange-600 dark:text-orange-400'
        }`}
      >
        {isConfident ? '✓ Confident' : '⚠ Unsure'} ({percentage}%)
      </span>
    )
  }

  const truncateChunk = (chunk: string, maxLength: number = 200): { text: string; isTruncated: boolean } => {
    if (chunk.length <= maxLength) {
      return { text: chunk, isTruncated: false }
    }
    return { text: chunk.slice(0, maxLength), isTruncated: true }
  }

  // Handle empty sources
  if (!sources || sources.length === 0) {
    return (
      <div className="mx-auto w-full mt-2">
        <div className="rounded-lg bg-main-view-fg/4 border border-dashed border-main-view-fg/10 p-3">
          <div className="flex items-center gap-3">
            <BookOpen className="size-4 text-main-view-fg/40" />
            <span className="text-sm text-main-view-fg/40">
              No relevant documents found in knowledge base
            </span>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div
      className="mx-auto w-full cursor-pointer break-words mt-2"
      onClick={handleClick}
    >
      <div className="rounded-lg bg-main-view-fg/4 border border-dashed border-main-view-fg/10 p-2">
        <div className="flex items-center gap-3">
          <BookOpen className="size-4 text-main-view-fg/60" />
          <button className="flex items-center gap-2 focus:outline-none">
            {isExpanded ? (
              <ChevronUp className="size-4 text-main-view-fg/60" />
            ) : (
              <ChevronDown className="size-4 text-main-view-fg/60" />
            )}
            <span className="font-medium text-main-view-fg/60 text-sm">
              📚 Sources ({sources.length} {sources.length === 1 ? 'document' : 'documents'})
            </span>
          </button>
        </div>

        {isExpanded && (
          <div className="mt-3 space-y-3">
            {sources.map((source, idx) => {
              const { text: chunkPreview, isTruncated } = truncateChunk(source.chunk)
              const workspaceName = getWorkspaceName(source)
              
              return (
                <div
                  key={idx}
                  className="ml-6 p-3 rounded-lg bg-main-view-fg/8 border border-main-view-fg/10"
                  onClick={(e) => e.stopPropagation()}
                >
                  <div className="space-y-2">
                    {/* Header with file name and confidence */}
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex-1 min-w-0">
                        <div className="font-medium text-main-view-fg text-sm truncate">
                          {source.file}
                        </div>
                        <div className="text-xs text-main-view-fg/50 mt-0.5">
                          {workspaceName} • Chunk #{source.chunk_index}
                        </div>
                      </div>
                      <div className="flex-shrink-0">
                        {getConfidenceBadge(source.score)}
                      </div>
                    </div>

                    {/* Chunk preview */}
                    <div className="text-sm text-main-view-fg/70 leading-relaxed">
                      <div className="whitespace-pre-wrap">{chunkPreview}</div>
                      {isTruncated && (
                        <button
                          className="text-blue-500 hover:text-blue-600 text-xs mt-1 inline-flex items-center gap-1"
                          onClick={(e) => {
                            e.stopPropagation()
                            // TODO: Implement full chunk expansion
                          }}
                        >
                          ... read more
                        </button>
                      )}
                    </div>

                    {/* File path */}
                    {source.metadata?.source_file && (
                      <div className="text-xs text-main-view-fg/40 font-mono truncate">
                        {source.metadata.source_file}
                      </div>
                    )}
                  </div>
                </div>
              )
            })}
          </div>
        )}
      </div>
    </div>
  )
})

SourcesBlock.displayName = 'SourcesBlock'

export default SourcesBlock
