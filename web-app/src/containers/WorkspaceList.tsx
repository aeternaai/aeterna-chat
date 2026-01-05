import { useState, useEffect } from 'react'
import { useWorkspace } from '@/hooks/useWorkspace'
import { useIndexingStore, useIndexingEventSetup } from '@/hooks/useIndexingStore'
import { WorkspaceVisibility } from '@janhq/core'
import { invoke } from '@tauri-apps/api/core'
import {
  IconFolder,
  IconFolderPlus,
  IconChevronDown,
  IconChevronRight,
  IconFile,
  IconTrash,
  IconPencil,
  IconPlus,
  IconCheck,
  IconX,
  IconAlertCircle,
  IconLock,
  IconWorld,
  IconLoader,
} from '@tabler/icons-react'
import { cn } from '@/lib/utils'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { toast } from 'sonner'
import { useTranslation } from '@/i18n/react-i18next-compat'

interface WorkspaceListProps {
  onAddFile?: (workspaceId: string) => void
}

export const WorkspaceList = ({ onAddFile }: WorkspaceListProps) => {
  const { t } = useTranslation()
  const indexingJobs = useIndexingStore((state) => state.indexingJobs)
  const removeIndexingJob = useIndexingStore((state) => state.removeIndexingJob)
  const {
    workspaces,
    currentWorkspaceId,
    workspaceFiles,
    isLoading,
    loadWorkspaces,
    setCurrentWorkspace,
    createWorkspace,
    updateWorkspace,
    deleteWorkspace,
    removeFileFromWorkspace,
    validateWorkspaceFiles,
  } = useWorkspace()

  const [expandedWorkspaces, setExpandedWorkspaces] = useState<Set<string>>(new Set())
  const [isCreating, setIsCreating] = useState(false)
  const [newWorkspaceName, setNewWorkspaceName] = useState('')
  const [editingWorkspaceId, setEditingWorkspaceId] = useState<string | null>(null)
  const [editName, setEditName] = useState('')

  // Setup event listeners for indexing progress
  useEffect(() => {
    useIndexingEventSetup()
  }, [])

  useEffect(() => {
    loadWorkspaces()
  }, [loadWorkspaces])

  const handleCreateWorkspace = async () => {
    if (!newWorkspaceName.trim()) {
      toast.error(t('workspace.error.nameRequired'))
      return
    }

    try {
      await createWorkspace(newWorkspaceName.trim())
      setNewWorkspaceName('')
      setIsCreating(false)
      toast.success(t('workspace.success.created'))
    } catch (error) {
      console.error('Failed to create workspace:', error)
      toast.error(t('workspace.error.createFailed'))
    }
  }

  const handleUpdateWorkspace = async (workspaceId: string) => {
    if (!editName.trim()) {
      toast.error(t('workspace.error.nameRequired'))
      return
    }

    const workspace = workspaces.find((w) => w.id === workspaceId)
    if (!workspace) return

    try {
      await updateWorkspace({
        ...workspace,
        name: editName.trim(),
      })
      setEditingWorkspaceId(null)
      setEditName('')
      toast.success(t('workspace.success.updated'))
    } catch (error) {
      console.error('Failed to update workspace:', error)
      toast.error(t('workspace.error.updateFailed'))
    }
  }

  const handleDeleteWorkspace = async (workspaceId: string) => {
    try {
      await deleteWorkspace(workspaceId)
      toast.success(t('workspace.success.deleted'))
    } catch (error) {
      console.error('Failed to delete workspace:', error)
      toast.error(t('workspace.error.deleteFailed'))
    }
  }

  const handleToggleWorkspace = (workspaceId: string) => {
    const newExpanded = new Set(expandedWorkspaces)
    if (newExpanded.has(workspaceId)) {
      newExpanded.delete(workspaceId)
    } else {
      newExpanded.add(workspaceId)
    }
    setExpandedWorkspaces(newExpanded)
    setCurrentWorkspace(workspaceId)
  }

  const handleRemoveFile = async (fileId: string) => {
    try {
      await removeFileFromWorkspace(fileId)
      toast.success(t('workspace.success.fileRemoved'))
    } catch (error) {
      console.error('Failed to remove file:', error)
      toast.error(t('workspace.error.removeFileFailed'))
    }
  }

  const handleValidateFiles = async (workspaceId: string) => {
    try {
      const statuses = await validateWorkspaceFiles(workspaceId)
      const invalidCount = statuses.filter((s) => !s.exists || !s.readable).length
      
      if (invalidCount === 0) {
        toast.success(t('workspace.success.allFilesValid'))
      } else {
        toast.warning(t('workspace.warning.invalidFiles', { count: invalidCount }))
      }
    } catch (error) {
      console.error('Failed to validate files:', error)
      toast.error(t('workspace.error.validateFailed'))
    }
  }

  if (isLoading) {
    return (
      <div className="px-3 py-2 text-sm text-muted-foreground">
        {t('workspace.loading')}
      </div>
    )
  }

  return (
    <div className="flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2">
        <div className="flex items-center gap-2">
          <IconFolder size={16} className="text-muted-foreground" />
          <span className="text-sm font-medium">{t('workspace.title')}</span>
        </div>
        <Button
          variant="default"
          className="h-6 w-6 p-0"
          onClick={() => setIsCreating(true)}
        >
          <IconFolderPlus size={16} />
        </Button>
      </div>

      {/* Create new workspace */}
      {isCreating && (
        <div className="px-3 py-1 flex items-center gap-2">
          <Input
            value={newWorkspaceName}
            onChange={(e) => setNewWorkspaceName(e.target.value)}
            placeholder={t('workspace.placeholder.name')}
            className="h-7 text-sm"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleCreateWorkspace()
              if (e.key === 'Escape') {
                setIsCreating(false)
                setNewWorkspaceName('')
              }
            }}
          />
          <Button
            variant="default"
            className="h-6 w-6 p-0 shrink-0"
            onClick={handleCreateWorkspace}
          >
            <IconCheck size={14} />
          </Button>
          <Button
            variant="default"
            className="h-6 w-6 p-0 shrink-0"
            onClick={() => {
              setIsCreating(false)
              setNewWorkspaceName('')
            }}
          >
            <IconX size={14} />
          </Button>
        </div>
      )}

      {/* Workspace list */}
      <div className="flex flex-col gap-1 px-2">
        {workspaces.map((workspace) => {
          const isExpanded = expandedWorkspaces.has(workspace.id)
          const files = workspaceFiles[workspace.id] || []
          const isEditing = editingWorkspaceId === workspace.id

          return (
            <div key={workspace.id} className="flex flex-col">
              {/* Workspace item */}
              <div
                className={cn(
                  'flex items-center gap-2 px-2 py-1.5 rounded-md hover:bg-accent cursor-pointer group',
                  currentWorkspaceId === workspace.id && 'bg-accent'
                )}
              >
                <button
                  onClick={() => handleToggleWorkspace(workspace.id)}
                  className="flex items-center gap-2 flex-1 min-w-0"
                >
                  {isExpanded ? (
                    <IconChevronDown size={14} className="shrink-0" />
                  ) : (
                    <IconChevronRight size={14} className="shrink-0" />
                  )}
                  <IconFolder size={14} className="shrink-0 text-muted-foreground" />
                  
                  {isEditing ? (
                    <Input
                      value={editName}
                      onChange={(e) => setEditName(e.target.value)}
                      className="h-6 text-xs"
                      autoFocus
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') handleUpdateWorkspace(workspace.id)
                        if (e.key === 'Escape') {
                          setEditingWorkspaceId(null)
                          setEditName('')
                        }
                      }}
                      onClick={(e) => e.stopPropagation()}
                    />
                  ) : (
                    <span className="text-xs truncate flex-1 text-left">
                      {workspace.name}
                    </span>
                  )}
                  
                  {workspace.visibility === WorkspaceVisibility.Private ? (
                    <IconLock size={12} className="shrink-0 text-muted-foreground opacity-50" />
                  ) : (
                    <IconWorld size={12} className="shrink-0 text-muted-foreground opacity-50" />
                  )}
                </button>

                {/* Actions */}
                {isEditing ? (
                  <div className="flex items-center gap-1">
                    <Button
                      variant="default"
                      className="h-5 w-5 p-0 opacity-0 group-hover:opacity-100"
                      onClick={() => handleUpdateWorkspace(workspace.id)}
                    >
                      <IconCheck size={12} />
                    </Button>
                    <Button
                      variant="default"
                      className="h-5 w-5 p-0 opacity-0 group-hover:opacity-100"
                      onClick={() => {
                        setEditingWorkspaceId(null)
                        setEditName('')
                      }}
                    >
                      <IconX size={12} />
                    </Button>
                  </div>
                ) : (
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        variant="default"
                        className="h-5 w-5 p-0 opacity-0 group-hover:opacity-100"
                      >
                        <IconPencil size={12} />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem
                        onClick={() => {
                          setEditingWorkspaceId(workspace.id)
                          setEditName(workspace.name)
                        }}
                      >
                        <IconPencil size={14} className="mr-2" />
                        {t('common:rename')}
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={() => onAddFile?.(workspace.id)}>
                        <IconPlus size={14} className="mr-2" />
                        {t('workspace.addFile')}
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={() => handleValidateFiles(workspace.id)}>
                        <IconAlertCircle size={14} className="mr-2" />
                        {t('workspace.validateFiles')}
                      </DropdownMenuItem>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem
                        onClick={() => handleDeleteWorkspace(workspace.id)}
                        className="text-destructive"
                      >
                        <IconTrash size={14} className="mr-2" />
                        {t('common:delete')}
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                )}
              </div>

              {/* Files list */}
              {isExpanded && files.length > 0 && (
                <div className="ml-6 flex flex-col gap-0.5 mt-1">
                  {files.map((file) => {
                    const indexingJob = indexingJobs[file.id]
                    const isIndexing = indexingJob?.status === 'processing'
                    const indexingError = indexingJob?.status === 'failed' ? indexingJob.error : undefined
                    
                    return (
                      <div
                        key={file.id}
                        className="flex items-center gap-2 px-2 py-1 rounded-md hover:bg-accent group"
                      >
                        <IconFile size={12} className="shrink-0 text-muted-foreground" />
                        <div className="flex-1 min-w-0">
                          <span className="text-xs truncate block" title={file.file_path}>
                            {file.name}
                          </span>
                          {/* Indexing progress bar */}
                          {isIndexing && (
                            <div className="w-full bg-accent rounded-full h-1 mt-0.5 overflow-hidden">
                              <div
                                className="bg-primary h-full transition-all duration-300"
                                style={{ width: `${indexingJob.percent}%` }}
                              />
                            </div>
                          )}
                        </div>

                        {/* Status indicators */}
                        {isIndexing && (
                          <div className="flex items-center gap-1 opacity-100 group-hover:opacity-100">
                            <IconLoader size={12} className="shrink-0 text-primary animate-spin" />
                            <span className="text-xs text-primary whitespace-nowrap">
                              {Math.round(indexingJob.percent)}%
                            </span>
                          </div>
                        )}
                        
                        {indexingError && (
                          <TooltipProvider>
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <IconAlertCircle size={12} className="shrink-0 text-destructive" />
                              </TooltipTrigger>
                              <TooltipContent side="right" className="max-w-xs">
                                <p className="text-xs">{indexingError}</p>
                              </TooltipContent>
                            </Tooltip>
                          </TooltipProvider>
                        )}

                        {!file.is_valid && !isIndexing && !indexingError && (
                          <IconAlertCircle size={12} className="shrink-0 text-destructive" />
                        )}

                        {/* Action buttons */}
                        <div className="flex items-center gap-1">
                          {isIndexing && (
                            <Button
                              variant="default"
                              className="h-4 w-4 p-0"
                              onClick={async () => {
                                try {
                                  await invoke('cancel_workspace_file_indexing', {
                                    fileId: file.id,
                                  })
                                  removeIndexingJob(file.id)
                                  toast.success('Indexing cancelled')
                                } catch (error) {
                                  console.error('Failed to cancel indexing:', error)
                                  toast.error('Failed to cancel indexing')
                                }
                              }}
                              title="Cancel indexing"
                            >
                              <IconX size={10} />
                            </Button>
                          )}
                          
                          {!isIndexing && (
                            <Button
                              variant="default"
                              className="h-4 w-4 p-0 opacity-0 group-hover:opacity-100"
                              onClick={() => handleRemoveFile(file.id)}
                            >
                              <IconTrash size={10} />
                            </Button>
                          )}
                        </div>
                      </div>
                    )
                  })}
                </div>
              )}

              {isExpanded && files.length === 0 && (
                <div className="ml-6 px-2 py-1 text-xs text-muted-foreground">
                  {t('workspace.noFiles')}
                </div>
              )}
            </div>
          )
        })}

        {workspaces.length === 0 && !isCreating && (
          <div className="px-2 py-3 text-xs text-center text-muted-foreground">
            {t('workspace.empty')}
          </div>
        )}
      </div>
    </div>
  )
}
