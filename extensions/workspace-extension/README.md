# Workspace Extension

This extension provides workspace management functionality for Jan, enabling users to organize and manage file references across different workspaces.

## Features

- **Workspace Management**: Create, read, update, and delete workspaces
- **File Reference Management**: Add, remove, and track file references (not copies)
- **File Validation**: Verify that referenced files still exist and are accessible
- **Event System**: Emit events for all workspace and file operations
- **Abstraction Layer**: Easy to swap implementations (filesystem, database, cloud storage, etc.)

## Architecture

### Storage Structure

```
file://workspaces/
├── workspace-id-1/
│   ├── workspace.json      # Workspace metadata
│   └── files/
│       ├── file-1.json     # File reference
│       └── file-2.json     # File reference
└── workspace-id-2/
    ├── workspace.json
    └── files/
        └── file-3.json
```

### Key Concepts

- **Workspaces**: Logical containers for organizing file references
  - Can be marked as private or public (property only, no access control implemented)
  - Store metadata like name, description, creation time, etc.

- **File References**: Pointers to files, not copies
  - Store original file path (`file://` URI)
  - Track metadata (name, size, mime type, etc.)
  - Can be validated to check if original file still exists

### Abstraction Layer

The extension follows Jan's extension pattern:

1. **Core Types** (`core/src/types/workspace/`):
   - `Workspace`, `WorkspaceFile`, `WorkspaceFileStatus`
   - `WorkspaceInterface` - Abstract interface definition

2. **Core Extension** (`core/src/browser/extension.ts`):
   - `WorkspaceExtension` - Abstract base class
   - Defines all CRUD methods as abstract

3. **Implementation** (`extensions/workspace-extension/src/index.ts`):
   - `JanWorkspaceExtension` - Concrete filesystem implementation
   - Can be swapped for database, cloud storage, etc.

## Usage

### Creating a Workspace

```typescript
import { Workspace, WorkspaceVisibility } from '@janhq/core'

const workspace: Workspace = {
  id: 'my-workspace',
  name: 'My Workspace',
  description: 'A workspace for my documents',
  visibility: WorkspaceVisibility.Private,
  created_at: Date.now(),
  updated_at: Date.now(),
}

await workspaceExtension.createWorkspace(workspace)
```

### Adding a File Reference

```typescript
import { WorkspaceFile } from '@janhq/core'

const fileRef: WorkspaceFile = {
  id: 'file-1',
  workspace_id: 'my-workspace',
  file_path: 'file:///Users/username/Documents/document.pdf',
  name: 'document.pdf',
  extension: 'pdf',
  size: 1024000,
  mime_type: 'application/pdf',
  added_at: Date.now(),
  updated_at: Date.now(),
  is_valid: true,
}

await workspaceExtension.addFileToWorkspace(fileRef)
```

### Validating File References

```typescript
// Validate a single file
const status = await workspaceExtension.validateFileReference('file-1')
console.log(status.exists, status.readable)

// Validate all files in a workspace
const statuses = await workspaceExtension.validateWorkspaceFiles('my-workspace')
statuses.forEach(status => {
  console.log(`File ${status.file_id}: ${status.exists ? 'valid' : 'invalid'}`)
})
```

## Events

The extension emits the following events:

- `onWorkspaceCreated` - When a workspace is created
- `onWorkspaceUpdated` - When a workspace is updated
- `onWorkspaceDeleted` - When a workspace is deleted
- `onFileAdded` - When a file reference is added
- `onFileRemoved` - When a file reference is removed
- `onFileUpdated` - When a file reference is updated
- `onFileValidated` - When a file reference is validated

## Future Enhancements

- **Access Control**: Implement actual private/public workspace permissions
- **Sync Workers**: Background service to monitor file changes
- **Document Processing**: Integration with RAG extension for indexing
- **Cloud Storage**: Alternative implementation using cloud storage backends
- **Database Storage**: Alternative implementation using SQLite or other databases
