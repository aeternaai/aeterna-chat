# Router Extension: Tauri API Fix

## Problem
The router extension was unable to detect the Tauri context even after implementing retry logic. Error: "Tauri context not available after 10 attempts"

## Root Cause
Extensions bundled with webpack cannot access `window.__TAURI__` directly. The extension was trying to use:
```typescript
(window as any).__TAURI__.core.invoke
```

This approach NEVER works in bundled extensions, regardless of timing or retry logic.

## Solution
Switch to the standard Tauri package import pattern used by other extensions (like llamacpp-extension):

```typescript
import { invoke } from '@tauri-apps/api/core'
```

## Changes Made

### 1. Added Dependency
**File:** `extensions/router-extension/package.json`
```json
"dependencies": {
  "@janhq/core": "../../core/package.tgz",
  "@tauri-apps/api": "2.8.0",  // ← Added
  "ts-loader": "^9.5.0"
}
```

### 2. Updated Imports
**File:** `extensions/router-extension/src/index.ts`
```typescript
import { invoke } from '@tauri-apps/api/core'
```

### 3. Removed Unnecessary Code
Deleted the following helper functions (no longer needed):
- `getInvoke()` - Was trying to capture window.__TAURI__.core.invoke
- `waitForTauri()` - Was polling for __TAURI__ to be injected
- Old `isTauriContext()` - Was checking window.__TAURI__ existence

### 4. Simplified Context Detection
**New approach:**
```typescript
/**
 * Check if running in Tauri context by attempting to use invoke
 */
async function isTauriContext(): Promise<boolean> {
  try {
    // If invoke exists and works, we're in Tauri
    await invoke('get_router_health')
    return true
  } catch (error) {
    // Either not in Tauri, or Python router not ready
    return false
  }
}
```

This function is only used during initial health check and isn't strictly necessary since `checkPythonRouterWithRetry()` already catches errors.

### 5. Simplified onLoad()
**Before:**
```typescript
async onLoad() {
  // 10 lines of Tauri detection logging
  const tauriReady = await waitForTauri(10, 100)
  
  if (tauriReady && isTauriContext()) {
    this.pythonRouterAvailable = await this.checkPythonRouterWithRetry(5, 1000)
  } else {
    console.log('Running in web mode')
    this.pythonRouterAvailable = false
  }
}
```

**After:**
```typescript
async onLoad() {
  console.log('[RouterExtension] 🚀 Checking for Python router...')
  this.pythonRouterAvailable = await this.checkPythonRouterWithRetry(5, 1000)
  // Simplified - just try to connect, let errors bubble naturally
}
```

### 6. Updated route() Method
**Before:**
```typescript
const invoke = getInvoke()
if (this.usePythonRouter && this.pythonRouterAvailable && invoke) {
  const decision = await invoke('route_request', { request })
}
```

**After:**
```typescript
if (this.usePythonRouter && this.pythonRouterAvailable) {
  const decision = await invoke('route_request', { request })
}
```

### 7. Updated checkPythonRouterWithRetry()
**Before:**
```typescript
private async checkPythonRouterWithRetry(...) {
  const invoke = getInvoke()
  if (!invoke) {
    console.warn('Tauri invoke not available')
    return false
  }
  // ... rest of logic
}
```

**After:**
```typescript
private async checkPythonRouterWithRetry(...) {
  // Just use invoke directly - if it fails, we're not in Tauri
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      await invoke('get_router_health')
      return true
    } catch (error) {
      // Retry or fail
    }
  }
}
```

## Why This Works

### Webpack Bundling
Extensions are bundled with webpack, which:
1. Creates an isolated scope for the extension code
2. Doesn't have access to global window properties injected by Tauri runtime
3. Requires proper module imports for external APIs

### @tauri-apps/api Package
This package:
1. Provides proper TypeScript types
2. Handles Tauri context detection internally
3. Works correctly in bundled code
4. Is the official/supported way to use Tauri APIs

### Pattern Used by Other Extensions
Looking at `llamacpp-extension/src/backend.ts`:
```typescript
import { invoke } from '@tauri-apps/api/core'
import { dirname, basename } from '@tauri-apps/api/path'
import { getSystemInfo } from '@janhq/tauri-plugin-hardware-api'
```

This is the established pattern throughout the Jan codebase.

## Testing
After rebuild:
1. Install dependency: `yarn install`
2. Build extension: `yarn build`
3. Run app: `make dev`
4. Check logs for:
   - "🚀 Checking for Python router..."
   - "🎯 Python router ready" (should appear quickly)
   - "✅ ROUTER MODE: PYTHON" (confirms Python router active)
5. Send a test message to trigger routing
6. Verify log shows "🐍 Using PYTHON router service"

## Related Files
- `extensions/router-extension/src/index.ts` - Main changes
- `extensions/router-extension/package.json` - Added dependency
- `extensions/llamacpp-extension/src/backend.ts` - Reference implementation

## Lessons Learned
1. **Don't access window.__TAURI__ in bundled extensions** - Use package imports
2. **Follow existing patterns** - Check how other extensions do it (llamacpp, etc.)
3. **Trust the tooling** - @tauri-apps/api handles context detection properly
4. **Retry logic helps** - But can't fix fundamentally wrong approach
5. **Read the full error context** - Timing wasn't the issue, the access method was

## Previous Attempts
- ✗ Module-load-time capture of invoke - Failed (too early)
- ✗ Runtime getInvoke() with window.__TAURI__ - Failed (not available in bundle)
- ✗ waitForTauri() with 10 retries - Failed (was never going to work)
- ✓ Import from @tauri-apps/api/core - SUCCESS

This matches the Jan architecture guide's extension system pattern.
