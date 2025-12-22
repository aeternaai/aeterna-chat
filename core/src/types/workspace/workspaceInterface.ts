import { Workspace, WorkspaceFile, WorkspaceFileStatus } from './workspaceEntity'

/**
 * Workspace extension interface for managing workspaces and their file references.
 * Provides abstraction layer for different workspace storage implementations.
 * @extends BaseExtension
 */
export interface WorkspaceInterface {
  /**
   * Creates a new workspace.
   * @param {Workspace} workspace - The workspace object to be created.
   * @returns {Promise<void>} A promise that resolves when the workspace has been created.
   */
  createWorkspace(workspace: Workspace): Promise<void>

  /**
   * Updates an existing workspace.
   * @param {Workspace} workspace - The workspace object with updated properties.
   * @returns {Promise<void>} A promise that resolves when the workspace has been updated.
   */
  updateWorkspace(workspace: Workspace): Promise<void>

  /**
   * Deletes an existing workspace.
   * @param {string} workspaceId - The ID of the workspace to be deleted.
   * @returns {Promise<void>} A promise that resolves when the workspace has been deleted.
   */
  deleteWorkspace(workspaceId: string): Promise<void>

  /**
   * Retrieves a specific workspace by ID.
   * @param {string} workspaceId - The ID of the workspace to retrieve.
   * @returns {Promise<Workspace | null>} A promise that resolves to the workspace or null if not found.
   */
  getWorkspace(workspaceId: string): Promise<Workspace | null>

  /**
   * Retrieves all existing workspaces.
   * @returns {Promise<Workspace[]>} A promise that resolves to an array of all workspaces.
   */
  getWorkspaces(): Promise<Workspace[]>

  /**
   * Adds a file reference to a workspace.
   * @param {WorkspaceFile} file - The file reference to be added.
   * @returns {Promise<void>} A promise that resolves when the file has been added.
   */
  addFileToWorkspace(file: WorkspaceFile): Promise<void>

  /**
   * Removes a file reference from a workspace.
   * @param {string} fileId - The ID of the file reference to remove.
   * @returns {Promise<void>} A promise that resolves when the file has been removed.
   */
  removeFileFromWorkspace(fileId: string): Promise<void>

  /**
   * Updates a file reference in a workspace.
   * @param {WorkspaceFile} file - The file reference with updated properties.
   * @returns {Promise<void>} A promise that resolves when the file has been updated.
   */
  updateWorkspaceFile(file: WorkspaceFile): Promise<void>

  /**
   * Retrieves a specific file reference by ID.
   * @param {string} fileId - The ID of the file reference to retrieve.
   * @returns {Promise<WorkspaceFile | null>} A promise that resolves to the file reference or null if not found.
   */
  getWorkspaceFile(fileId: string): Promise<WorkspaceFile | null>

  /**
   * Retrieves all file references for a specific workspace.
   * @param {string} workspaceId - The ID of the workspace.
   * @returns {Promise<WorkspaceFile[]>} A promise that resolves to an array of file references.
   */
  getWorkspaceFiles(workspaceId: string): Promise<WorkspaceFile[]>

  /**
   * Validates that a file reference still points to an existing, accessible file.
   * @param {string} fileId - The ID of the file reference to validate.
   * @returns {Promise<WorkspaceFileStatus>} A promise that resolves to the validation status.
   */
  validateFileReference(fileId: string): Promise<WorkspaceFileStatus>

  /**
   * Validates all file references in a workspace.
   * @param {string} workspaceId - The ID of the workspace.
   * @returns {Promise<WorkspaceFileStatus[]>} A promise that resolves to an array of validation statuses.
   */
  validateWorkspaceFiles(workspaceId: string): Promise<WorkspaceFileStatus[]>
}
