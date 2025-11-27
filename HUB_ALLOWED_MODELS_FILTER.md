# Hub Allowed Models Filter

## Overview

This feature adds a toggle to the Hub section that filters and displays only the models that are configured as "allowed download models" in the General settings. This helps administrators and users control which models can be downloaded from the Hub catalog with an intuitive add/edit/delete UI.

## Implementation Summary

### 1. State Management - General Settings Hook (`web-app/src/hooks/useGeneralSetting.ts`)

#### Added State Property
```typescript
type GeneralSettingState = {
  // ... existing properties
  allowedDownloadModels?: string
  setAllowedDownloadModels: (value: string) => void
}
```

#### Default Value and Setter
```typescript
{
  allowedDownloadModels: undefined,
  setAllowedDownloadModels: (value) => set({ allowedDownloadModels: value }),
}
```

The setting is automatically persisted to `localStorage` via zustand's `persist` middleware.

### 2. UI - General Settings Page (`web-app/src/routes/settings/general.tsx`)

#### Added Interface
```typescript
interface AllowedDownloadModel {
  id: string
}
```

#### Added State Management
```typescript
const [allowedModels, setAllowedModels] = useState<AllowedDownloadModel[]>([])
const [editingModelIndex, setEditingModelIndex] = useState<number | null>(null)
const [newModel, setNewModel] = useState<AllowedDownloadModel>({ id: '' })
```

#### Load/Save Functions
- `useEffect` to parse JSON from settings and populate the list
- `saveAllowedModels()` to serialize list back to JSON
- `handleAddModel()` to add new models
- `handleEditModel()` to enter edit mode
- `handleSaveEdit()` to save edits
- `handleCancelEdit()` to cancel editing
- `handleDeleteModel()` to remove models

#### UI Card Component
Similar to Router's "Model Routing Configuration" with:
- List of existing allowed models with Edit/Delete buttons
- Inline editing mode when Edit is clicked
- Add new model form with dashed border
- Helper text with tips
- All changes auto-save to localStorage

**Key Features**:
- ✅ **Add**: Enter model ID and click "Add Model"
- ✅ **Edit**: Click "Edit" button, modify ID, click "Save" or "Cancel"
- ✅ **Delete**: Click "Delete" button to remove
- ✅ **Auto-save**: All changes immediately persist
- ✅ **Visual feedback**: Different styles for view/edit modes

### 3. Frontend - Hub Component (`web-app/src/routes/hub/index.tsx`)

#### Removed Router Dependency
- Removed `import { RouterManager } from '@janhq/core'`
- Now uses `useGeneralSetting` hook directly

#### Added Setting Access
```typescript
const allowedDownloadModels = useGeneralSetting((state) => state.allowedDownloadModels)
```

#### Updated Filter Logic
Added JSON parsing and filtering in the `filteredModels` useMemo:

```typescript
// Apply allowed models filter
if (showOnlyAllowed) {
  if (allowedDownloadModels) {
    try {
      const allowedList = JSON.parse(allowedDownloadModels) as Array<{ id: string }>
      if (Array.isArray(allowedList) && allowedList.length > 0) {
        const allowedIds = allowedList.map(item => item.id).filter(id => typeof id === 'string')
        if (allowedIds.length > 0) {
          filtered = filtered
            ?.map((model) => ({
              ...model,
              quants: model.quants.filter((variant) =>
                allowedIds.includes(variant.model_id)
              ),
            }))
            .filter((model) => model.quants.length > 0)
        }
      }
    } catch (error) {
      console.error('[Hub] Failed to parse allowed download models:', error)
    }
  }
}
```

## JSON Format

The allowed download models setting uses a JSON array format:

```json
[
  {"id": "Qwen3-VL-8B-Instruct-IQ4_XS"},
  {"id": "gemma-3n-E4B-it-IQ4_XS"},
  {"id": "Phi-4-mini-instruct_Q4_K_M"}
]
```

**Key Points:**
- Must be valid JSON
- Array of objects with `id` property
- `id` should match the exact model variant ID (including quantization)
- Empty or invalid JSON = allows all models

## How It Works

### Configuration Flow
```
User enters JSON in General Settings
    ↓
Saved to localStorage via zustand
    ↓
Hub component reads setting
    ↓
Parses JSON to extract model IDs
    ↓
Filters catalog models by variant IDs
    ↓
Displays filtered results
```

### Filter Behavior

1. **When toggle is OFF** (default):
   - Shows all models from the Hub catalog
   - User can browse and download any model

2. **When toggle is ON**:
   - Parses the `allowedDownloadModels` JSON setting
   - Shows only model variants that match the allowed IDs
   - Helps administrators control which models users can access
   - Each model card shows only the variants that are in the allowed list

3. **Error Handling**:
   - Invalid JSON is caught and logged to console
   - Falls back to showing all models if parsing fails
   - Gracefully handles empty or malformed data

### Example

If your setting is:
```json
[
  {"id": "Qwen3-VL-8B-Instruct-IQ4_XS"},
  {"id": "gemma-3n-E4B-it-IQ4_XS"}
]
```

Then enabling the "Allowed Models" toggle will:
- Show only models that have `Qwen3-VL-8B-Instruct-IQ4_XS` or `gemma-3n-E4B-it-IQ4_XS` variants
- Filter out all other model variants
- Make it easy to control what users can download

## Integration with Existing Features

### Compatible with Other Filters
- **Search**: Can be combined with search - searches within allowed models
- **Downloaded filter**: Can be used together - shows allowed models that are also downloaded
- **Sort options**: Works with newest/most-downloaded sorting

### Clears HuggingFace Repo
When toggling on, it clears any manually added HuggingFace repo to avoid confusion, similar to the "Downloaded" filter behavior.

## Files Modified

1. **`web-app/src/hooks/useGeneralSetting.ts`**
   - Added `allowedDownloadModels` state property
   - Added `setAllowedDownloadModels` setter function
   - Automatically persists to localStorage

2. **`web-app/src/routes/settings/general.tsx`**
   - Added allowed download models input field in "Others" card
   - Added state management for the setting
   - Provides UI for JSON configuration

3. **`web-app/src/routes/hub/index.tsx`**
   - Added `allowedDownloadModels` from useGeneralSetting
   - Added filter logic with JSON parsing in `filteredModels` useMemo
   - Added dependency to useMemo
   - Toggle uses new general setting instead of router configs

4. **`extensions/router-extension/src/index.ts`** (previous implementation, no longer used by Hub)
   - Added `getAllowedModelIds()` method (kept for future use)

## Build & Deploy

No build required for these changes - they're all in the web-app:

```bash
# If app is running, it should hot-reload automatically
# Otherwise, start the app
make dev
```

## Testing Checklist

- [x] General settings hook updated without errors
- [x] General settings page has no TypeScript errors  
- [x] Hub component has no TypeScript errors
- [x] Setting automatically persists via zustand
- [ ] UI appears in General Settings as separate "Allowed Download Models" card
- [ ] Can add new models via the form
- [ ] Can edit existing models
- [ ] Can delete models
- [ ] Changes auto-save to localStorage
- [ ] Toggle in Hub filters models correctly
- [ ] Toggle shows all models when list is empty
- [ ] Works with search functionality
- [ ] Works with downloaded filter
- [ ] Works with sort options
- [ ] No console errors in browser

## Usage

### Configure Allowed Models

1. Open Jan application
2. Navigate to **Settings → General**
3. Scroll to the "Allowed Download Models" card (after "Others" section)
4. **To add a model**:
   - Enter the model ID in the input field
   - Click "Add Model" button
5. **To edit a model**:
   - Click the "Edit" button on any model
   - Modify the ID
   - Click "Save" or "Cancel"
6. **To delete a model**:
   - Click the "Delete" button on any model
7. Changes are saved automatically to localStorage

### Use the Filter

1. Navigate to **Hub** section
2. Look for the "Allowed Models" toggle (next to "Downloaded")
3. Click the toggle to enable filtering
4. Only configured models will be shown

## Architecture

### Data Flow
```
User Input (General Settings)
    ↓
useGeneralSetting Hook (zustand)
    ↓
localStorage (persist middleware)
    ↓
Hub Component reads setting
    ↓
JSON.parse() extracts IDs
    ↓
Filter catalog models
    ↓
Display filtered results
```

### Component Integration
```
useGeneralSetting (State Management)
    ↓
General Settings Page (Configuration UI)
    ↓
Hub Component (Filter Logic)
    ↓
filteredModels (Computed Results)
    ↓
Virtual List Rendering
```

## Differences from Router Config

**Previous approach** (router-based):
- Allowed models defined in router extension settings
- Used for routing decisions AND Hub filtering
- Tied to router functionality

**New approach** (general settings):
- Allowed download models separate from router
- Purely for Hub download control
- Independent of routing functionality
- More flexible for administrators

This separation allows:
- Router to have its own set of models for routing
- Hub to have a different set for downloads
- Better control over what users can access

## Future Enhancements

Potential improvements:
1. Add visual indicator showing how many models are filtered
2. Add "Import from Router" button to copy router configs
3. Add validation for JSON format with error messages
4. Show tooltip explaining which models are allowed
5. Add bulk download for all allowed models
6. Add UI builder for JSON instead of raw text input
7. Support wildcards or patterns (e.g., "Qwen*", "*-IQ4_XS")
8. Add model categories or tags for easier configuration
