import {
  WorkspaceExtension,
  Workspace,
  WorkspaceFile,
  WorkspaceFileStatus,
  WorkspaceEvent,
  events,
} from '@janhq/core'
import { invoke } from '@tauri-apps/api/core'

/**
 * Default implementation of WorkspaceExtension.
 * Uses database storage via Tauri commands for workspace management.
 * Provides persistence and efficient querying through SQLite.
 */
export default class JanWorkspaceExtension extends WorkspaceExtension {
  async onLoad() {
    // Database is initialized on app startup
    // No need for directory setup
  }

  onUnload(): void {
    // Cleanup if needed
  }

  async createWorkspace(workspace: Workspace): Promise<void> {
    // Call Tauri command to create workspace in database
    await invoke('create_workspace', { workspace })
    
    // Emit event
    events.emit(WorkspaceEvent.OnWorkspaceCreated, workspace)
  }

  async updateWorkspace(workspace: Workspace): Promise<void> {
    // Update timestamp
    workspace.updated_at = Date.now()
    
    // Call Tauri command to update workspace in database
    await invoke('update_workspace', { workspace })
    
    // Emit event
    events.emit(WorkspaceEvent.OnWorkspaceUpdated, workspace)
  }

  async deleteWorkspace(workspaceId: string): Promise<void> {
    // Call Tauri command to delete workspace (CASCADE deletes files)
    await invoke('delete_workspace', { workspaceId })
    
    // Emit event
    events.emit(WorkspaceEvent.OnWorkspaceDeleted, { id: workspaceId })
  }

  async getWorkspace(workspaceId: string): Promise<Workspace | null> {
    // Call Tauri command to get workspace from database
    const workspace = await invoke<Workspace | null>(
      'get_workspace',
      { workspaceId }
    )
    
    return workspace
  }

  async getWorkspaces(): Promise<Workspace[]> {
    // Call Tauri command to list all workspaces from database
    const workspaces = await invoke<Workspace[]>('list_workspaces', {})
    
    return workspaces
  }

  async addFileToWorkspace(file: WorkspaceFile): Promise<void> {
    // Call Tauri command to add file to workspace in database
    await invoke('add_workspace_file', {
      workspaceId: file.workspace_id,
      filePath: file.file_path,
      name: file.name,
    })
    
    // Emit event
    events.emit(WorkspaceEvent.OnFileAdded, file)
  }

  async removeFileFromWorkspace(fileId: string): Promise<void> {
    // Call Tauri command to remove file from workspace
    await invoke('remove_workspace_file', { fileId })
    
    // Emit event
    events.emit(WorkspaceEvent.OnFileRemoved, { fileId })
  }

  async updateWorkspaceFile(file: WorkspaceFile): Promise<void> {
    // Update timestamp
    file.updated_at = Date.now()
    
    // Call Tauri command to update file in database
    await invoke('update_workspace_file', { file })
    
    // Emit event
    events.emit(WorkspaceEvent.OnFileUpdated, file)
  }

  async getWorkspaceFile(fileId: string): Promise<WorkspaceFile | null> {
    // Get all workspaces and search for the file
    // TODO: Add a dedicated get_workspace_file command to backend for efficiency
    const workspaces = await this.getWorkspaces()
    
    for (const workspace of workspaces) {
      const files = await this.getWorkspaceFiles(workspace.id)
      const file = files.find((f) => f.id === fileId)
      if (file) {
        return file
      }
    }
    
    return null
  }

  async getWorkspaceFiles(workspaceId: string): Promise<WorkspaceFile[]> {
    // Call Tauri command to list files in workspace
    const files = await invoke<WorkspaceFile[]>(
      'list_workspace_files',
      { workspaceId }
    )
    
    return files
  }

  async validateFileReference(fileId: string): Promise<WorkspaceFileStatus> {
    // Call Tauri command to validate file
    const status = await invoke<WorkspaceFileStatus>(
      'validate_workspace_file',
      { fileId }
    )
    
    // Emit event if validation successful
    if (status.exists && status.readable) {
      events.emit(WorkspaceEvent.OnFileValidated, status)
    }
    
    return status
  }

  async validateWorkspaceFiles(workspaceId: string): Promise<WorkspaceFileStatus[]> {
    const files = await this.getWorkspaceFiles(workspaceId)
    const statuses: WorkspaceFileStatus[] = []
    
    for (const file of files) {
      const status = await this.validateFileReference(file.id)
      statuses.push(status)
    }
    
    return statuses
  }
}
