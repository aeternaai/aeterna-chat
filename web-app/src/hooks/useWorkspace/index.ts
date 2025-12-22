import { create } from 'zustand'
import {
  Workspace,
  WorkspaceFile,
  WorkspaceVisibility,
  WorkspaceFileStatus,
  ExtensionTypeEnum,
  BaseExtension,
} from '@janhq/core'
import { ExtensionManager } from '@/lib/extension'

interface WorkspaceState {
  workspaces: Workspace[]
  currentWorkspaceId?: string
  workspaceFiles: Record<string, WorkspaceFile[]>
  isLoading: boolean
  
  // Workspace CRUD
  createWorkspace: (name: string, description?: string, visibility?: WorkspaceVisibility) => Promise<void>
  updateWorkspace: (workspace: Workspace) => Promise<void>
  deleteWorkspace: (workspaceId: string) => Promise<void>
  getWorkspace: (workspaceId: string) => Promise<Workspace | null>
  loadWorkspaces: () => Promise<void>
  setCurrentWorkspace: (workspaceId?: string) => void
  
  // File management
  addFileToWorkspace: (workspaceId: string, filePath: string) => Promise<void>
  removeFileFromWorkspace: (fileId: string) => Promise<void>
  loadWorkspaceFiles: (workspaceId: string) => Promise<void>
  validateWorkspaceFiles: (workspaceId: string) => Promise<WorkspaceFileStatus[]>
}

export const useWorkspace = create<WorkspaceState>((set, get) => ({
  workspaces: [],
  currentWorkspaceId: undefined,
  workspaceFiles: {},
  isLoading: false,

  createWorkspace: async (name, description, visibility = WorkspaceVisibility.Private) => {
    const extension = ExtensionManager.getInstance()
      .getAll()
      .find((ext: BaseExtension) => ext.type?.() === ExtensionTypeEnum.Workspace)

    if (!extension) {
      console.error('Workspace extension not found')
      return
    }

    const workspace: Workspace = {
      id: `workspace-${Date.now()}`,
      name,
      description,
      visibility,
      created_at: Date.now(),
      updated_at: Date.now(),
    }

    await (extension as any).createWorkspace(workspace)
    await get().loadWorkspaces()
  },

  updateWorkspace: async (workspace) => {
    const extension = ExtensionManager.getInstance()
      .getAll()
      .find((ext: BaseExtension) => ext.type?.() === ExtensionTypeEnum.Workspace)

    if (!extension) {
      console.error('Workspace extension not found')
      return
    }

    await (extension as any).updateWorkspace(workspace)
    await get().loadWorkspaces()
  },

  deleteWorkspace: async (workspaceId) => {
    const extension = ExtensionManager.getInstance()
      .getAll()
      .find((ext: BaseExtension) => ext.type?.() === ExtensionTypeEnum.Workspace)

    if (!extension) {
      console.error('Workspace extension not found')
      return
    }

    await (extension as any).deleteWorkspace(workspaceId)
    
    // Clear current workspace if it was deleted
    if (get().currentWorkspaceId === workspaceId) {
      set({ currentWorkspaceId: undefined })
    }
    
    await get().loadWorkspaces()
  },

  getWorkspace: async (workspaceId) => {
    const extension = ExtensionManager.getInstance()
      .getAll()
      .find((ext: BaseExtension) => ext.type?.() === ExtensionTypeEnum.Workspace)

    if (!extension) {
      console.error('Workspace extension not found')
      return null
    }

    return await (extension as any).getWorkspace(workspaceId)
  },

  loadWorkspaces: async () => {
    set({ isLoading: true })
    
    const extension = ExtensionManager.getInstance()
      .getAll()
      .find((ext: BaseExtension) => ext.type?.() === ExtensionTypeEnum.Workspace)

    if (!extension) {
      console.error('Workspace extension not found')
      set({ isLoading: false })
      return
    }

    try {
      const workspaces = await (extension as any).getWorkspaces()
      set({ workspaces, isLoading: false })
    } catch (error) {
      console.error('Failed to load workspaces:', error)
      set({ isLoading: false })
    }
  },

  setCurrentWorkspace: (workspaceId) => {
    set({ currentWorkspaceId: workspaceId })
    
    // Load files for the selected workspace
    if (workspaceId) {
      get().loadWorkspaceFiles(workspaceId)
    }
  },

  addFileToWorkspace: async (workspaceId, filePath) => {
    const extension = ExtensionManager.getInstance()
      .getAll()
      .find((ext: BaseExtension) => ext.type?.() === ExtensionTypeEnum.Workspace)

    if (!extension) {
      console.error('Workspace extension not found')
      return
    }

    // Extract file name from path
    const pathParts = filePath.split('/')
    const fileName = pathParts[pathParts.length - 1]
    const extension_part = fileName.includes('.') ? fileName.split('.').pop() : undefined

    const file: WorkspaceFile = {
      id: `file-${Date.now()}`,
      workspace_id: workspaceId,
      file_path: filePath,
      name: fileName,
      extension: extension_part,
      added_at: Date.now(),
      updated_at: Date.now(),
      is_valid: true,
    }

    await (extension as any).addFileToWorkspace(file)
    await get().loadWorkspaceFiles(workspaceId)
  },

  removeFileFromWorkspace: async (fileId) => {
    const extension = ExtensionManager.getInstance()
      .getAll()
      .find((ext: BaseExtension) => ext.type?.() === ExtensionTypeEnum.Workspace)

    if (!extension) {
      console.error('Workspace extension not found')
      return
    }

    await (extension as any).removeFileFromWorkspace(fileId)
    
    // Reload files for current workspace
    const { currentWorkspaceId } = get()
    if (currentWorkspaceId) {
      await get().loadWorkspaceFiles(currentWorkspaceId)
    }
  },

  loadWorkspaceFiles: async (workspaceId) => {
    const extension = ExtensionManager.getInstance()
      .getAll()
      .find((ext: BaseExtension) => ext.type?.() === ExtensionTypeEnum.Workspace)

    if (!extension) {
      console.error('Workspace extension not found')
      return
    }

    try {
      const files = await (extension as any).getWorkspaceFiles(workspaceId)
      set((state) => ({
        workspaceFiles: {
          ...state.workspaceFiles,
          [workspaceId]: files,
        },
      }))
    } catch (error) {
      console.error('Failed to load workspace files:', error)
    }
  },

  validateWorkspaceFiles: async (workspaceId) => {
    const extension = ExtensionManager.getInstance()
      .getAll()
      .find((ext: BaseExtension) => ext.type?.() === ExtensionTypeEnum.Workspace)

    if (!extension) {
      console.error('Workspace extension not found')
      return []
    }

    try {
      const statuses = await (extension as any).validateWorkspaceFiles(workspaceId)
      
      // Reload files after validation
      await get().loadWorkspaceFiles(workspaceId)
      
      return statuses
    } catch (error) {
      console.error('Failed to validate workspace files:', error)
      return []
    }
  },
}))
