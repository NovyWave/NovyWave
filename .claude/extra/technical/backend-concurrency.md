# Backend Concurrency Patterns

Guide for maintaining thread-safe concurrent access in NovyWave's backend.

## Global State Architecture

The backend has **7 global static stores** using `once_cell::Lazy`:

| Store | Type | Purpose |
|-------|------|---------|
| `PARSING_SESSIONS` | `Mutex<HashMap<String, Arc<Mutex<f32>>>>` | Progress tracking (0.0-1.0) |
| `WAVEFORM_DATA_STORE` | `Mutex<HashMap<String, WaveformData>>` | Parsed waveform data |
| `WAVEFORM_METADATA_STORE` | `Mutex<HashMap<String, WaveformMetadata>>` | Lightweight file metadata |
| `VCD_LOADING_IN_PROGRESS` | `Mutex<HashSet<String>>` | Loading state tracker |
| `LOADING_NOTIFIERS` | `Mutex<HashMap<String, Arc<Notify>>>` | Async completion notifications |
| `transition_cache` | `RwLock<BTreeMap<...>>` | Signal transition cache |
| `cache_stats` | `RwLock<CacheStats>` | Cache hit/miss statistics |

## Lock Ordering Convention

**Always acquire locks in this order to prevent deadlocks:**

```
PARSING_SESSIONS → WAVEFORM_DATA_STORE → WAVEFORM_METADATA_STORE
    → VCD_LOADING_IN_PROGRESS → LOADING_NOTIFIERS
    → transition_cache → cache_stats
```

## Critical Antipatterns

### 1. Sequential Lock-Release-Lock (TOCTOU)

```rust
// ❌ RACE CONDITION: Lock released between operations
{
    let store = WAVEFORM_DATA_STORE.lock()?;
    if store.contains_key(file) { return Ok(()); }
}  // Lock released here

// Another thread can modify state HERE

{
    let mut loading = VCD_LOADING_IN_PROGRESS.lock()?;
    loading.insert(file);  // May double-load!
}
```

```rust
// ✅ CORRECT: Acquire all locks atomically before any check
let store_guard = WAVEFORM_DATA_STORE.lock()?;
let mut loading_guard = VCD_LOADING_IN_PROGRESS.lock()?;

if store_guard.contains_key(file) { return Ok(()); }
if loading_guard.contains(file) { /* wait */ }
loading_guard.insert(file);

// Drop locks only after all decisions made
```

### 2. Check-Then-Act Without Lock

```rust
// ❌ RACE: check and act are not atomic
if !cache.read()?.contains_key(&id) {
    // Another thread can insert HERE
    let data = expensive_load();
    cache.write()?.insert(id, data);  // Duplicate work!
}
```

```rust
// ✅ Double-checked locking pattern
{ let cache = self.transition_cache.read()?;
  if let Some(data) = cache.get(&id) { return Ok(data.clone()); }
}

let mut cache = self.transition_cache.write()?;
// Re-check after acquiring write lock
if let Some(data) = cache.get(&id) { return Ok(data.clone()); }

let data = self.load_internal()?;
cache.insert(id, data.clone());
Ok(data)
```

### 3. `.unwrap()` on Lock Acquisition

```rust
// ❌ Cascading failures: one panic poisons lock, all others panic
let cache = self.transition_cache.read().unwrap();
```

```rust
// ✅ Recover from poisoned locks
let cache = match self.transition_cache.read() {
    Ok(guard) => guard,
    Err(poisoned) => poisoned.into_inner(),
};
```

### 4. CPU Work Without spawn_blocking

```rust
// ❌ Blocks async runtime (potentially seconds)
let body_result = std::panic::catch_unwind(|| {
    wellen::viewers::read_body(header, &options)  // CPU-intensive!
});
```

```rust
// ✅ Offload CPU work to blocking thread pool
let result = tokio::task::spawn_blocking(move || {
    std::panic::catch_unwind(|| {
        wellen::viewers::read_body(header, &options)
    })
}).await?;
```

**Note:** The wellen library's parsing functions take borrows (e.g., `header.body`, `&header.hierarchy`) that are not `Send`. This makes wrapping with `spawn_blocking` complex - data must be cloned or restructured to move into the closure. Currently, only `read_header_from_file` (line 1094) is wrapped. Other parsing calls remain synchronous; consider refactoring wellen usage if async runtime blocking becomes an issue.

### 5. Nested Mutex for Simple Counters

```rust
// ❌ Risk of deadlock, unnecessary overhead
type Progress = Arc<Mutex<f32>>;

// ✅ Use atomics for simple values
type Progress = Arc<AtomicU32>;  // 0-1000 for 0.1% precision
progress.store((pct * 1000.0) as u32, Ordering::Release);
```

## Correct Patterns

### Atomic Multi-Lock Acquisition

When modifying multiple stores, acquire ALL locks before modifying ANY:

```rust
fn reset_runtime_state_for_workspace() {
    // Acquire all locks FIRST
    let sessions_guard = PARSING_SESSIONS.lock();
    let store_guard = WAVEFORM_DATA_STORE.lock();
    let metadata_guard = WAVEFORM_METADATA_STORE.lock();
    let loading_guard = VCD_LOADING_IN_PROGRESS.lock();
    let notifiers_guard = LOADING_NOTIFIERS.lock();
    let cache_guard = SIGNAL_CACHE_MANAGER.transition_cache.write();
    let stats_guard = SIGNAL_CACHE_MANAGER.cache_stats.write();

    // THEN modify all stores
    if let Ok(mut s) = sessions_guard { s.clear(); }
    if let Ok(mut s) = store_guard { s.clear(); }
    // ... etc

    // All locks released together here
}
```

### Notify-Based Coordination

For async waiting on file load completion:

```rust
// Waiting thread
let notifier = {
    let mut notifiers = LOADING_NOTIFIERS.lock()?;
    notifiers.entry(file.to_string())
        .or_insert_with(|| Arc::new(Notify::new()))
        .clone()
};
drop(notifiers);  // Release lock before await

match timeout(Duration::from_secs(30), notifier.notified()).await {
    Ok(()) => { /* loading complete */ }
    Err(_) => { /* timeout */ }
}

// Completing thread
fn complete_loading(file_path: &str) {
    let loading_guard = VCD_LOADING_IN_PROGRESS.lock();
    let notifiers_guard = LOADING_NOTIFIERS.lock();

    if let Ok(mut loading) = loading_guard { loading.remove(file_path); }
    if let Ok(mut notifiers) = notifiers_guard {
        if let Some(notifier) = notifiers.remove(file_path) {
            notifier.notify_waiters();
        }
    }
}
```

## Audit Commands

Find potential race conditions:

```bash
# Find sequential lock-release-lock patterns (TOCTOU)
grep -n "\.lock()" backend/src/main.rs | head -50

# Find .unwrap() on locks (poison vulnerability)
grep -n "\.lock()\.unwrap()\|\.read()\.unwrap()\|\.write()\.unwrap()" backend/src/main.rs

# Find catch_unwind without spawn_blocking (blocking async)
grep -B5 "catch_unwind" backend/src/main.rs | grep -v "spawn_blocking"

# Find nested Mutex types
grep -n "Mutex<.*Mutex" backend/src/main.rs
```

## Testing Concurrent Behavior

Race conditions are non-deterministic. Test with:

1. **Multiple browser tabs** - opens multiple WebSocket sessions
2. **Concurrent file loads** - load same file from different tabs
3. **Rapid workspace switching** - while files are loading
4. **Large files** - maximizes race window during parsing

## Additional Protected Operations

### Config File Operations

Config file read-modify-write operations are protected by `CONFIG_FILE_LOCK`:

```rust
// Serialize config file operations
static CONFIG_FILE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn update_workspace_history_on_select(root: &Path) -> GlobalSection {
    let _config_guard = match CONFIG_FILE_LOCK.lock() { ... };
    let mut global = read_global_section();
    global.workspace_history.touch_path(...);
    save_global_section(global)
}
```

### File Loading Deduplication

`load_waveform_file` now uses `VCD_LOADING_IN_PROGRESS` to deduplicate concurrent load requests for the same file. Subsequent requests wait for the first load to complete via `LOADING_NOTIFIERS`.

## Lower-Priority Issues (Documented)

These issues are documented but not critical to fix:

1. **Signal source lock during I/O**: ✅ FIXED - Lock scope reduced to release immediately after `load_signals()`, processing happens outside lock
2. **Plugin manager lock contention**: Watcher callbacks acquire `PLUGIN_MANAGER.lock()` - may cause delays under rapid file changes. Fix requires channel-based refactor (queue events for processing instead of direct lock acquisition during WASM plugin calls)
3. **Workspace root stale read**: `WorkspaceContext::root()` can return stale value during `set_root()` - paths remain valid
