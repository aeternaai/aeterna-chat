/**
 * Workspace events that can be emitted by the workspace extension.
 */
export enum WorkspaceEvent {
  /** Emitted when a workspace is created. */
  OnWorkspaceCreated = 'onWorkspaceCreated',
  /** Emitted when a workspace is updated. */
  OnWorkspaceUpdated = 'onWorkspaceUpdated',
  /** Emitted when a workspace is deleted. */
  OnWorkspaceDeleted = 'onWorkspaceDeleted',
  /** Emitted when a file is added to a workspace. */
  OnFileAdded = 'onFileAdded',
  /** Emitted when a file is removed from a workspace. */
  OnFileRemoved = 'onFileRemoved',
  /** Emitted when a file reference is updated. */
  OnFileUpdated = 'onFileUpdated',
  /** Emitted when file validation is complete. */
  OnFileValidated = 'onFileValidated',
}
