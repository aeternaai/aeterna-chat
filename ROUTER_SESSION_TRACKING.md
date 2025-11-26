# Router Session Tracking - Support for Same Model ID

## Problem

**Question**: What if I want to use the same model for both routing and answering?

**Example Scenario**:
- Router model: `Phi-4-mini-instruct_Q4_K_M` (for making routing decisions)
- Response model: `Phi-4-mini-instruct_Q4_K_M` (also answering some questions)

**Previous Limitation**:
The original fix exempted models from auto-unload by **model_id** only:
```typescript
.filter((id) => id !== this.routerModelId)
```

**Issue**: If you load the same model twice (once for routing, once for response), the filter would:
- Exempt BOTH sessions from auto-unload (memory waste), OR
- Unload BOTH sessions (breaking the router)

## Solution: Session-Based Tracking

Track the **specific session PID** of the router model instead of just the model ID.

### Architecture

Each time a model is loaded in Jan, it gets:
- A unique **PID** (process ID) from the llama-server process
- A unique **port** number
- A unique **API key**

These are tracked in a `SessionInfo` object:
```typescript
interface SessionInfo {
  pid: number          // Unique process identifier
  port: number         // HTTP port for this instance
  model_id: string     // Model identifier (can be same for multiple sessions)
  api_key: string
  is_embedding: boolean
}
```

### Implementation

#### 1. Extension: Track Router Session PID

**File:** `extensions/llamacpp-extension/src/index.ts`

```typescript
export default class llamacpp_extension extends AIEngine {
  provider: string = 'llamacpp'
  autoUnload: boolean = true
  
  // Router configuration
  routerModelId: string = 'Phi-4-mini-instruct_Q4_K_M'
  routerSessionPid: number | null = null  // ← NEW: Track specific session
  
  // ... rest of class
}
```

#### 2. Methods to Manage Router Session

```typescript
/**
 * Mark a specific session as the router session to exempt from auto-unload
 * This allows using the same model for both routing and answering
 * @param modelId - Model ID to mark as router
 */
async setRouterSession(modelId: string): Promise<void> {
  try {
    const sessionInfo = await this.findSessionByModel(modelId)
    if (sessionInfo) {
      this.routerSessionPid = sessionInfo.pid
      logger.info(
        `Marked session PID ${sessionInfo.pid} (model: ${modelId}) as router session - will be exempt from auto-unload`
      )
    } else {
      logger.warn(
        `Cannot mark router session: No active session found for model ${modelId}`
      )
    }
  } catch (error) {
    logger.error(`Failed to set router session for ${modelId}:`, error)
  }
}

/**
 * Clear the router session marker
 */
clearRouterSession(): void {
  logger.info(`Clearing router session marker (was PID ${this.routerSessionPid})`)
  this.routerSessionPid = null
}
```

#### 3. Enhanced Auto-Unload Logic

```typescript
const nonEmbeddingModels: string[] = sessionInfos
  .filter(
    (s): s is SessionInfo => s !== null && s.is_embedding === false
  )
  .map((s) => s.model_id)
  .filter((id) => {
    // If we have a router session PID, exempt that specific session
    if (this.routerSessionPid !== null) {
      const sessionInfo = sessionInfos.find(s => s?.model_id === id)
      if (sessionInfo?.pid === this.routerSessionPid) {
        return false // Exempt this specific session
      }
    }
    // Fallback: also exempt by model_id (for cases where router PID not tracked)
    return id !== this.routerModelId
  })

if (nonEmbeddingModels.length > 0) {
  logger.info(
    `Auto-unloading ${nonEmbeddingModels.length} models (preserving router session: PID=${this.routerSessionPid}, model=${this.routerModelId})`
  )
  await Promise.all(
    nonEmbeddingModels.map((modelId) => this.unload(modelId))
  )
}
```

#### 4. Frontend: Mark Router Session After Loading

**File:** `web-app/src/providers/DataProvider.tsx`

```typescript
serviceHub.models().startModel(llamacppProvider, ROUTER_MODEL_ID)
  .then(() => {
    console.log(`[DataProvider] Router model '${ROUTER_MODEL_ID}' loaded successfully`)
    
    // Mark this session as the router session to exempt from auto-unload
    // This is critical if using the same model for both routing and answering
    if (llamacppProvider && typeof (llamacppProvider as any).setRouterSession === 'function') {
      ;(llamacppProvider as any).setRouterSession(ROUTER_MODEL_ID).catch((error: Error) => {
        console.warn('[DataProvider] Failed to mark router session:', error)
      })
    }
  })
```

## How It Works

### Scenario 1: Different Models (Original Use Case)

**Setup**:
- Router: `Phi-4-mini-instruct_Q4_K_M` (PID 12345)
- Response: `Qwen3-VL-8B-Instruct-IQ4_XS` (PID 67890)

**Auto-Unload Behavior**:
1. Load router model → PID 12345 → `routerSessionPid = 12345`
2. User sends message → Router selects `Qwen3-VL-8B-Instruct-IQ4_XS`
3. Load response model → Auto-unload checks:
   - Session 12345 (`Phi-4-mini-instruct_Q4_K_M`) → PID matches `routerSessionPid` → **KEEP**
   - Session 67890 (`Qwen3-VL-8B-Instruct-IQ4_XS`) → PID doesn't match → Can be unloaded later

**Result**: ✅ Router model stays loaded

### Scenario 2: Same Model for Both (New Capability)

**Setup**:
- Router: `Phi-4-mini-instruct_Q4_K_M` (PID 12345)
- Response: `Phi-4-mini-instruct_Q4_K_M` (PID 67890) ← Different session, same model!

**Auto-Unload Behavior**:
1. Load router model → PID 12345 → `routerSessionPid = 12345`
2. User sends message → Router selects `Phi-4-mini-instruct_Q4_K_M` (for answering)
3. Load response model → Creates NEW session with PID 67890
4. Auto-unload checks:
   - Session 12345 (`Phi-4-mini-instruct_Q4_K_M`) → PID matches `routerSessionPid` → **KEEP**
   - Session 67890 (`Phi-4-mini-instruct_Q4_K_M`) → PID doesn't match → **UNLOAD**

**Result**: ✅ Router session preserved, response session unloaded (as intended)

### Scenario 3: Fallback - No PID Tracked

**Setup**:
- Router: `Phi-4-mini-instruct_Q4_K_M` (PID 12345)
- `routerSessionPid = null` (for some reason)

**Auto-Unload Behavior**:
1. Auto-unload checks both filters:
   - `routerSessionPid === null` → Skip PID check
   - Fall back to model_id check → `id !== this.routerModelId`
   - Any session with model_id `Phi-4-mini-instruct_Q4_K_M` → **KEEP**

**Result**: ✅ Graceful degradation to original behavior

## Benefits

### 1. Flexibility
- ✅ Can use same model for routing AND answering
- ✅ Supports multiple sessions of the same model
- ✅ Clear separation between "router role" and "response role"

### 2. Precision
- ✅ Exempts only the specific router session (by PID)
- ✅ Allows other sessions of the same model to be auto-unloaded
- ✅ No accidental protection of unrelated sessions

### 3. Backward Compatibility
- ✅ Falls back to model_id filtering if PID not available
- ✅ Works with existing single-model setups
- ✅ No breaking changes to existing behavior

### 4. Memory Efficiency
- ✅ Only router session stays loaded
- ✅ Response sessions can be unloaded as usual
- ✅ No memory waste from protecting all sessions of a model

## Usage Examples

### Example 1: Use Phi-4 for Everything

```typescript
// Router configuration
const ROUTER_MODEL = 'Phi-4-mini-instruct_Q4_K_M'

// Available models for answering (including router model)
const AVAILABLE_MODELS = [
  'Phi-4-mini-instruct_Q4_K_M',  // Can be selected for simple queries
  'Qwen3-VL-8B-Instruct-IQ4_XS', // For vision tasks
  'gemma-3n-E4B-it-IQ4_XS'       // For coding tasks
]

// Behavior:
// 1. Router session (PID 12345) stays loaded
// 2. If router selects Phi-4 for answering → New session (PID 67890) created
// 3. After response → Session 67890 can be unloaded
// 4. Router session 12345 remains for next routing decision
```

### Example 2: Manually Control Router Session

```typescript
// Load model
const session = await llamacppProvider.load('my-model')

// Mark as router session
await llamacppProvider.setRouterSession('my-model')

// Later, if you want to unmark it
llamacppProvider.clearRouterSession()
```

## Logs to Expect

### When Router Session is Marked
```
Marked session PID 12345 (model: Phi-4-mini-instruct_Q4_K_M) as router session - will be exempt from auto-unload
```

### During Auto-Unload
```
Auto-unloading 1 models (preserving router session: PID=12345, model=Phi-4-mini-instruct_Q4_K_M)
```

### If Same Model Loaded Twice
```
// First load (router)
Marked session PID 12345 (model: Phi-4-mini-instruct_Q4_K_M) as router session

// Second load (response)
Successfully loaded model Phi-4-mini-instruct_Q4_K_M with PID 67890

// Auto-unload
Checking session PID 12345 (Phi-4-mini-instruct_Q4_K_M) → matches routerSessionPid → KEEP
Checking session PID 67890 (Phi-4-mini-instruct_Q4_K_M) → PID mismatch → UNLOAD
Auto-unloading 1 models (preserving router session: PID=12345, model=Phi-4-mini-instruct_Q4_K_M)
```

## Future Enhancements

### 1. Purpose Tagging in SessionInfo

Extend the Rust `SessionInfo` struct to include purpose:
```rust
pub struct SessionInfo {
    pub pid: u32,
    pub port: u16,
    pub model_id: String,
    pub api_key: String,
    pub is_embedding: bool,
    pub purpose: Option<SessionPurpose>,  // NEW
}

pub enum SessionPurpose {
    Router,
    Chat,
    Embedding,
    Tool,
}
```

### 2. Multiple Router Models

Support multiple routing strategies with different models:
```typescript
routerSessions: Map<string, number> = new Map()  // strategy -> PID
```

### 3. Reference Counting

Track how many "roles" each session has:
```typescript
sessionRoles: Map<number, Set<'router' | 'chat' | 'tool'>> = new Map()
// Only unload when all roles removed
```

## Testing

### Test Case 1: Same Model, Different Sessions
1. Load `Phi-4` as router → PID 1000
2. Send query → Router selects `Phi-4` for answer
3. Load `Phi-4` for response → PID 2000
4. Verify: PID 1000 stays, PID 2000 can be unloaded

### Test Case 2: Router Session Persistence
1. Load router model → PID 1000
2. Load/unload several other models
3. Verify: PID 1000 never unloaded
4. Clear router session
5. Load new model → Verify PID 1000 can now be unloaded

### Test Case 3: Fallback Behavior
1. Don't call `setRouterSession`
2. Load model matching `routerModelId`
3. Verify: Model still exempt (fallback to model_id filter)

## Summary

This enhancement provides **session-level granularity** for auto-unload exemptions, enabling:
- Same model for multiple purposes
- Precise control over which sessions to preserve
- Better memory management
- Future-proof architecture for multi-role sessions
