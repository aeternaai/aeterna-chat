/**
 * Workspace type defines the shape of a workspace object.
 * @stored
 */

export enum WorkspaceVisibility {
  Private = 'private',
  Public = 'public',
}

export type Workspace = {
  /** Represents the unique identifier of the workspace. */
  id: string
  /** Represents the name of the workspace. */
  name: string
  /** Represents the description of the workspace. */
  description?: string
  /** Represents the visibility of the workspace (private/public). */
  visibility: WorkspaceVisibility
  /** Represents the creation timestamp of the workspace. */
  created_at: number
  /** Represents the last update timestamp of the workspace. */
  updated_at: number
  /** Represents the owner identifier of the workspace. */
  owner_id?: string
  /** Represents additional metadata of the workspace. */
  metadata?: Record<string, unknown>
}

/**
 * WorkspaceFile represents a file reference within a workspace.
 * Files are not copied, only referenced by their original path.
 */
export type WorkspaceFile = {
  /** Represents the unique identifier of the file reference. */
  id: string
  /** Represents the workspace id this file belongs to. */
  workspace_id: string
  /** Represents the original file path (file:// URI). */
  file_path: string
  /** Represents the display name of the file. */
  name: string
  /** Represents the file extension. */
  extension?: string
  /** Represents the file size in bytes. */
  size?: number
  /** Represents the MIME type of the file. */
  mime_type?: string
  /** Represents when the file reference was added. */
  added_at: number
  /** Represents when the file reference was last updated. */
  updated_at: number
  /** Represents additional metadata. */
  metadata?: Record<string, unknown>
  /** Represents whether the file still exists at the original path. */
  is_valid?: boolean
}

/**
 * WorkspaceFileStatus represents the validation status of a file reference.
 */
export type WorkspaceFileStatus = {
  /** The file reference id. */
  file_id: string
  /** Whether the file exists at the original path. */
  exists: boolean
  /** Whether the file is readable. */
  readable: boolean
  /** Last check timestamp. */
  checked_at: number
  /** Error message if validation failed. */
  error?: string
}
