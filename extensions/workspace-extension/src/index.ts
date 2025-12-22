import {
  WorkspaceExtension,
  Workspace,
  WorkspaceFile,
  WorkspaceFileStatus,
  WorkspaceEvent,
  fs,
  joinPath,
  events,
} from '@janhq/core'

/**
 * Default implementation of WorkspaceExtension.
 * Stores workspaces and file references in the file system using JSON.
 * Provides abstraction layer for easy switching to different storage backends.
 */
export default class JanWorkspaceExtension extends WorkspaceExtension {
  private workspacesDir = 'file://workspaces'
  
  async onLoad() {
    // Ensure workspaces directory exists
    if (!(await fs.existsSync(this.workspacesDir))) {
      await fs.mkdir(this.workspacesDir)
    }
  }

  onUnload(): void {
    // Cleanup if needed
  }

  async createWorkspace(workspace: Workspace): Promise<void> {
    const workspacePath = await this.getWorkspacePath(workspace.id)
    const workspaceFolder = await joinPath([this.workspacesDir, workspace.id])
    
    // Create workspace folder
    if (!(await fs.existsSync(workspaceFolder))) {
      await fs.mkdir(workspaceFolder)
    }
    
    // Create files folder for this workspace
    const filesFolder = await joinPath([workspaceFolder, 'files'])
    if (!(await fs.existsSync(filesFolder))) {
      await fs.mkdir(filesFolder)
    }
    
    // Save workspace metadata
    await fs.writeFileSync(workspacePath, JSON.stringify(workspace, null, 2))
    
    // Emit event
    events.emit(WorkspaceEvent.OnWorkspaceCreated, workspace)
  }

  async updateWorkspace(workspace: Workspace): Promise<void> {
    const workspacePath = await this.getWorkspacePath(workspace.id)
    
    // Check if workspace exists
    if (!(await fs.existsSync(workspacePath))) {
      throw new Error(`Workspace ${workspace.id} not found`)
    }
    
    // Update timestamp
    workspace.updated_at = Date.now()
    
    // Save updated workspace
    await fs.writeFileSync(workspacePath, JSON.stringify(workspace, null, 2))
    
    // Emit event
    events.emit(WorkspaceEvent.OnWorkspaceUpdated, workspace)
  }

  async deleteWorkspace(workspaceId: string): Promise<void> {
    const workspaceFolder = await joinPath([this.workspacesDir, workspaceId])
    
    // Check if workspace exists
    if (!(await fs.existsSync(workspaceFolder))) {
      throw new Error(`Workspace ${workspaceId} not found`)
    }
    
    // Delete the entire workspace folder (including all file references)
    await fs.rm(workspaceFolder)
    
    // Emit event
    events.emit(WorkspaceEvent.OnWorkspaceDeleted, { id: workspaceId })
  }

  async getWorkspace(workspaceId: string): Promise<Workspace | null> {
    const workspacePath = await this.getWorkspacePath(workspaceId)
    
    if (!(await fs.existsSync(workspacePath))) {
      return null
    }
    
    try {
      const workspaceData = await fs.readFileSync(workspacePath)
      return JSON.parse(workspaceData) as Workspace
    } catch (error) {
      console.error(`Failed to read workspace ${workspaceId}:`, error)
      return null
    }
  }

  async getWorkspaces(): Promise<Workspace[]> {
    if (!(await fs.existsSync(this.workspacesDir))) {
      return []
    }
    
    const workspaceFolders = await fs.readdirSync(this.workspacesDir)
    const workspaces: Workspace[] = []
    
    for (const folder of workspaceFolders) {
      const workspacePath = await joinPath([
        this.workspacesDir,
        folder,
        'workspace.json',
      ])
      
      if (!(await fs.existsSync(workspacePath))) {
        continue
      }
      
      try {
        const workspaceData = await fs.readFileSync(workspacePath)
        workspaces.push(JSON.parse(workspaceData) as Workspace)
      } catch (error) {
        console.error(`Failed to read workspace ${folder}:`, error)
      }
    }
    
    return workspaces
  }

  async addFileToWorkspace(file: WorkspaceFile): Promise<void> {
    const filePath = await this.getWorkspaceFilePath(file.workspace_id, file.id)
    const filesFolder = await joinPath([
      this.workspacesDir,
      file.workspace_id,
      'files',
    ])
    
    // Ensure files folder exists
    if (!(await fs.existsSync(filesFolder))) {
      await fs.mkdir(filesFolder)
    }
    
    // Save file reference
    await fs.writeFileSync(filePath, JSON.stringify(file, null, 2))
    
    // Emit event
    events.emit(WorkspaceEvent.OnFileAdded, file)
  }

  async removeFileFromWorkspace(fileId: string): Promise<void> {
    // Need to find the file in all workspaces
    const workspaces = await this.getWorkspaces()
    
    for (const workspace of workspaces) {
      const filePath = await this.getWorkspaceFilePath(workspace.id, fileId)
      
      if (await fs.existsSync(filePath)) {
        await fs.rm(filePath)
        
        // Emit event
        events.emit(WorkspaceEvent.OnFileRemoved, { 
          fileId, 
          workspaceId: workspace.id 
        })
        return
      }
    }
    
    throw new Error(`File ${fileId} not found in any workspace`)
  }

  async updateWorkspaceFile(file: WorkspaceFile): Promise<void> {
    const filePath = await this.getWorkspaceFilePath(file.workspace_id, file.id)
    
    // Check if file reference exists
    if (!(await fs.existsSync(filePath))) {
      throw new Error(`File ${file.id} not found in workspace ${file.workspace_id}`)
    }
    
    // Update timestamp
    file.updated_at = Date.now()
    
    // Save updated file reference
    await fs.writeFileSync(filePath, JSON.stringify(file, null, 2))
    
    // Emit event
    events.emit(WorkspaceEvent.OnFileUpdated, file)
  }

  async getWorkspaceFile(fileId: string): Promise<WorkspaceFile | null> {
    // Need to search through all workspaces
    const workspaces = await this.getWorkspaces()
    
    for (const workspace of workspaces) {
      const filePath = await this.getWorkspaceFilePath(workspace.id, fileId)
      
      if (await fs.existsSync(filePath)) {
        try {
          const fileData = await fs.readFileSync(filePath)
          return JSON.parse(fileData) as WorkspaceFile
        } catch (error) {
          console.error(`Failed to read file ${fileId}:`, error)
        }
      }
    }
    
    return null
  }

  async getWorkspaceFiles(workspaceId: string): Promise<WorkspaceFile[]> {
    const filesFolder = await joinPath([
      this.workspacesDir,
      workspaceId,
      'files',
    ])
    
    if (!(await fs.existsSync(filesFolder))) {
      return []
    }
    
    const fileNames = await fs.readdirSync(filesFolder)
    const files: WorkspaceFile[] = []
    
    for (const fileName of fileNames) {
      const filePath = await joinPath([filesFolder, fileName])
      
      try {
        const fileData = await fs.readFileSync(filePath)
        files.push(JSON.parse(fileData) as WorkspaceFile)
      } catch (error) {
        console.error(`Failed to read file ${fileName}:`, error)
      }
    }
    
    return files
  }

  async validateFileReference(fileId: string): Promise<WorkspaceFileStatus> {
    const file = await this.getWorkspaceFile(fileId)
    
    if (!file) {
      return {
        file_id: fileId,
        exists: false,
        readable: false,
        checked_at: Date.now(),
        error: 'File reference not found',
      }
    }
    
    // Check if the original file exists and is readable
    try {
      const exists = await fs.existsSync(file.file_path)
      
      if (!exists) {
        return {
          file_id: fileId,
          exists: false,
          readable: false,
          checked_at: Date.now(),
          error: 'Original file not found',
        }
      }
      
      // Try to read the file to check readability
      await fs.readFileSync(file.file_path)
      
      const status: WorkspaceFileStatus = {
        file_id: fileId,
        exists: true,
        readable: true,
        checked_at: Date.now(),
      }
      
      // Update file reference with validation status
      file.is_valid = true
      file.updated_at = Date.now()
      await this.updateWorkspaceFile(file)
      
      // Emit event
      events.emit(WorkspaceEvent.OnFileValidated, status)
      
      return status
    } catch (error) {
      const status: WorkspaceFileStatus = {
        file_id: fileId,
        exists: false,
        readable: false,
        checked_at: Date.now(),
        error: error instanceof Error ? error.message : 'Unknown error',
      }
      
      // Update file reference with validation status
      if (file) {
        file.is_valid = false
        file.updated_at = Date.now()
        await this.updateWorkspaceFile(file)
      }
      
      return status
    }
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

  // Helper methods
  
  private async getWorkspacePath(workspaceId: string): Promise<string> {
    return await joinPath([
      this.workspacesDir,
      workspaceId,
      'workspace.json',
    ])
  }

  private async getWorkspaceFilePath(
    workspaceId: string,
    fileId: string
  ): Promise<string> {
    return await joinPath([
      this.workspacesDir,
      workspaceId,
      'files',
      `${fileId}.json`,
    ])
  }
}
