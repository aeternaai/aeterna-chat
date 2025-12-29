/**
 * Tauri Projects Service - Database implementation
 */

import { invoke } from '@tauri-apps/api/core'
import { ulid } from 'ulidx'
import type { ProjectsService, ThreadFolder } from './types'

export class TauriProjectsService implements ProjectsService {
  async getProjects(): Promise<ThreadFolder[]> {
    try {
      const folders = await invoke<Array<{
        id: string
        name: string
        workspace_id?: string
        parent_folder_id?: string
        created_at: number
        updated_at: number
      }>>('list_folders', { workspaceId: null })

      // Convert database format to ThreadFolder format
      return folders.map(folder => ({
        id: folder.id,
        name: folder.name,
        updated_at: folder.updated_at,
      }))
    } catch (error) {
      console.error('Error getting projects from database:', error)
      return []
    }
  }

  async addProject(name: string): Promise<ThreadFolder> {
    try {
      const id = ulid()
      const now = Date.now()

      const folder = await invoke<{
        id: string
        name: string
        workspace_id?: string
        parent_folder_id?: string
        created_at: number
        updated_at: number
      }>('create_folder', {
        name,
        workspaceId: null,
        parentFolderId: null,
      })

      return {
        id: folder.id,
        name: folder.name,
        updated_at: folder.updated_at,
      }
    } catch (error) {
      console.error('Error adding project to database:', error)
      throw error
    }
  }

  async updateProject(id: string, name: string): Promise<void> {
    try {
      await invoke('update_folder', {
        folder: {
          id,
          name,
          updated_at: Date.now(),
        },
      })
    } catch (error) {
      console.error('Error updating project in database:', error)
      throw error
    }
  }

  async deleteProject(id: string): Promise<void> {
    try {
      await invoke('delete_folder', { folderId: id })
    } catch (error) {
      console.error('Error deleting project from database:', error)
      throw error
    }
  }

  async getProjectById(id: string): Promise<ThreadFolder | undefined> {
    try {
      const folder = await invoke<{
        id: string
        name: string
        workspace_id?: string
        parent_folder_id?: string
        created_at: number
        updated_at: number
      } | null>('get_folder', { folderId: id })

      if (!folder) return undefined

      return {
        id: folder.id,
        name: folder.name,
        updated_at: folder.updated_at,
      }
    } catch (error) {
      console.error('Error getting project by ID from database:', error)
      return undefined
    }
  }

  async setProjects(projects: ThreadFolder[]): Promise<void> {
    // Bulk update not implemented - use individual updates
    console.warn('setProjects: Bulk updates not supported, use individual updates')
  }
}
