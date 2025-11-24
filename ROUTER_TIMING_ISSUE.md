# Router Timing Issue Analysis

**Date:** November 23, 2025  
**Issue:** Tauri app falls back to TypeScript router despite Python service being available  
**Root Cause:** Race condition between extension loading and router service startup

---

## The Problem

The app is falling back to the TypeScript router **not because Python is unavailable**, but because of a **timing race condition** during startup.

---

## What's Happening (Startup Sequence)

### Timeline of Events

```
Time 0ms: App starts
    ├─ Tauri initialization begins
    ├─ Extensions begin loading
    │
Time 50-100ms: setup_router() called
    ├─ Spawns async background task (non-blocking!)
    │   └─ Task starts Python service
    │
Time 100-200ms: Router Extension onLoad() executes
    ├─ Calls invoke('get_router_health')
    ├─ Health check FAILS ❌ (Python not ready yet!)
    ├─ Sets pythonRouterAvailable = false
    └─ Console: "⚠️ ROUTER MODE: TYPESCRIPT FALLBACK"
    │
Time 500-2000ms: Python service actually starts
    ├─ Python executable found
    ├─ Process spawned
    ├─ FastAPI imports loaded
    ├─ Uvicorn server starts
    ├─ Binds to port 8765
    └─ Service is NOW ready ✅
    │
Time 2000ms+: User sends first message
    └─ Uses TypeScript router (pythonRouterAvailable was set to false earlier)
```

---

## Root Cause Analysis

### 1. Async Router Startup (Non-Blocking)

**File:** `src-tauri/src/core/setup.rs:259-268`

```rust
pub fn setup_router<R: Runtime>(app: &App<R>) {
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {  // ← ASYNC SPAWN (doesn't block!)
        log::info!("Starting router service...");
        if let Err(e) = start_router(...).await {
            log::error!("Failed to start router service: {}", e);
        } else {
            log::info!("Router service started successfully");
        }
    });  // ← Returns immediately, doesn't wait for completion
}
```

**Impact:** 
- `setup_router()` returns immediately
- App initialization continues
- Python service starts in background
- No guarantee of completion before extensions load

### 2. Extension Loads Too Early

**File:** `extensions/router-extension/src/index.ts:70-82`

```typescript
async onLoad() {
    // ... setup code ...
    
    // Check if Python router service is available (Tauri only)
    if (invoke) {
        try {
            await invoke('get_router_health')  // ← Checks IMMEDIATELY on load
            this.pythonRouterAvailable = true
            console.log('[RouterExtension] Python router service is available')
        } catch (error) {
            console.warn('[RouterExtension] Python router service not available, using fallback:', error)
            this.pythonRouterAvailable = false  // ← Sets to false permanently
        }
    }
}
```

**Impact:**
- Extension loads during app setup
- Health check happens before Python service is ready
- `pythonRouterAvailable` set to `false` based on premature check
- Never rechecks - decision is permanent for the session

### 3. Python Service Startup Time

**Typical startup time:** 1-3 seconds

**Breakdown:**
- 100-200ms: Find Python executable
- 200-500ms: Spawn process
- 500-1500ms: Import FastAPI, uvicorn, pydantic
- 200-500ms: Start uvicorn server, bind to port
- 100-300ms: First health check response ready

**Total:** ~1-3 seconds from `spawn()` to ready

### 4. Wait Logic Exists But Doesn't Help

**File:** `src-tauri/src/core/router/commands.rs:32`

```rust
// Wait for service to be ready
wait_for_router_ready(&config, 20).await?;  // 20 × 500ms = 10 seconds max
```

**Why this doesn't help:**
- This wait happens in the **background async task**
- Extension loading happens on the **main app initialization path**
- They run **in parallel**, not sequentially
- Extension checks health before the background task completes waiting

---

## Why This Happens

The current architecture has these characteristics:

1. **Non-blocking startup:** Router service starts asynchronously to avoid blocking app launch
2. **Eager extension loading:** Extensions load as soon as possible during app init
3. **No coordination:** No mechanism to ensure router is ready before extension checks
4. **No retry:** Extension health check happens once, never retries

This creates a race condition where:
- Fast machines might work (Python starts before extension loads)
- Slow machines always fail (Extension loads before Python ready)
- Results are non-deterministic

---

## Evidence from Code

### Router Service Path

```
App starts
    ↓
lib.rs:210 - setup::setup_router(app) called
    ↓
setup.rs:259 - setup_router() spawns async task (RETURNS IMMEDIATELY)
    ↓                                      ↓
App init continues                    Background task continues
    ↓                                      ↓
Extensions load                        start_router() called
    ↓                                      ↓
RouterExtension.onLoad()              start_router_service() spawns Python
    ↓                                      ↓
invoke('get_router_health')           wait_for_router_ready() (20 attempts)
    ↓                                      ↓
FAILS ❌ (too early!)                  Service becomes ready ✅
    ↓
pythonRouterAvailable = false
```

---

## Solutions

### Option 1: Add Retry Logic to Extension (Quick Fix) ⭐ RECOMMENDED

Modify the extension to retry health checks with backoff:

```typescript
async onLoad() {
    // ... existing setup ...
    
    if (invoke) {
        // Try multiple times with increasing delays
        this.pythonRouterAvailable = await this.checkPythonRouterWithRetry(5, 1000);
        
        if (this.pythonRouterAvailable) {
            console.log('✅ [RouterExtension] ROUTER MODE: PYTHON (Python service available on port 8765)')
        } else {
            console.log('⚠️  [RouterExtension] ROUTER MODE: TYPESCRIPT FALLBACK (Python service not available)')
        }
    }
}

private async checkPythonRouterWithRetry(
    maxAttempts: number, 
    delayMs: number
): Promise<boolean> {
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
        try {
            await invoke('get_router_health')
            console.log(`[RouterExtension] Python router ready (attempt ${attempt}/${maxAttempts})`)
            return true
        } catch (error) {
            if (attempt === maxAttempts) {
                console.warn(`[RouterExtension] Python router not available after ${maxAttempts} attempts:`, error)
                return false
            }
            // Wait before retrying
            await new Promise(resolve => setTimeout(resolve, delayMs))
        }
    }
    return false
}
```

**Pros:**
- Simple to implement
- No changes to Rust code
- Gives Python service time to start
- Deterministic (always waits enough time)

**Cons:**
- Delays extension loading by a few seconds
- Adds complexity to extension code

---

### Option 2: Block App Startup Until Router Ready (Thorough Fix)

Make router startup synchronous in setup:

```rust
pub fn setup_router<R: Runtime>(app: &App<R>) {
    let app_handle = app.handle().clone();
    
    // Block until router is ready OR fails
    tauri::async_runtime::block_on(async move {
        log::info!("Starting router service...");
        match start_router(app_handle.clone(), app_handle.state::<AppState>()).await {
            Ok(_) => log::info!("Router service started successfully"),
            Err(e) => log::warn!("Router service failed to start: {}", e),
        }
    });
}
```

**Pros:**
- Guarantees router is ready before extensions load
- Extension can trust health check result
- Clean architecture

**Cons:**
- Blocks app startup for 1-3 seconds
- Bad user experience if Python not installed
- Defeats purpose of async startup

---

### Option 3: Lazy Router Initialization (Deferred)

Don't check health on load, check on first routing request:

```typescript
async route(context: RouteContext): Promise<RouteDecision> {
    // Check Python router availability on-demand (first request only)
    if (this.usePythonRouter && this.pythonRouterAvailable === null && invoke) {
        try {
            await invoke('get_router_health')
            this.pythonRouterAvailable = true
        } catch {
            this.pythonRouterAvailable = false
        }
    }
    
    // Rest of routing logic...
}
```

**Pros:**
- No startup delay
- Router ready by time user sends first message
- Simple implementation

**Cons:**
- First routing request slightly slower
- Less predictable behavior
- Startup logs don't show router status

---

### Option 4: Event-Based Notification (Elegant)

Have router service emit event when ready:

**Rust side:**
```rust
pub async fn start_router(...) -> Result<(), String> {
    // Start service...
    wait_for_router_ready(&config, 20).await?;
    
    // Emit ready event
    app.emit("router-service-ready", ()).ok();
    
    Ok(())
}
```

**TypeScript side:**
```typescript
async onLoad() {
    if (invoke) {
        // Listen for ready event
        await window.__TAURI__.event.listen('router-service-ready', () => {
            this.pythonRouterAvailable = true
            console.log('✅ [RouterExtension] Python router is now ready!')
        })
        
        // Initial check
        try {
            await invoke('get_router_health')
            this.pythonRouterAvailable = true
        } catch {
            this.pythonRouterAvailable = false
            // Will be updated when event fires
        }
    }
}
```

**Pros:**
- Clean event-driven architecture
- No blocking, no delays
- Can update status dynamically

**Cons:**
- More complex implementation
- Requires event handling on both sides

---

## Recommended Solution

**Use Option 1 (Retry Logic) because:**

1. ✅ Quick to implement (only TypeScript changes)
2. ✅ Gives Python service adequate startup time (5 seconds)
3. ✅ Deterministic and reliable
4. ✅ No user-visible delay (happens during app load)
5. ✅ Gracefully handles Python not installed

**Implementation:**
- 5 retry attempts
- 1 second between attempts
- Total wait: up to 5 seconds
- Covers typical Python startup time (1-3 seconds)

---

## Testing the Fix

After implementing Option 1:

1. **Check logs for retry attempts:**
   ```
   [RouterExtension] Checking Python router... (attempt 1/5)
   [RouterExtension] Checking Python router... (attempt 2/5)
   [RouterExtension] Python router ready (attempt 3/5)
   ✅ [RouterExtension] ROUTER MODE: PYTHON
   ```

2. **Verify Python router is used:**
   ```
   🐍 [RouterExtension] Using PYTHON router service
   [RouterExtension] Python router: model-id (18ms) - ...
   ```

3. **Confirm fallback still works if Python unavailable:**
   - Uninstall Python dependencies
   - Should see all 5 attempts fail
   - Should fall back to TypeScript

---

## Conclusion

**The app falls back to TypeScript router due to a RACE CONDITION, not because Python is unavailable.**

- Router service starts asynchronously (doesn't block app)
- Extension checks health immediately on load (too early!)
- Python needs 1-3 seconds to start
- Extension checks before Python is ready
- Falls back to TypeScript permanently

**Fix:** Add retry logic to extension health check (wait up to 5 seconds for Python to start)

This will make the Python router work reliably on all machines, regardless of speed.
