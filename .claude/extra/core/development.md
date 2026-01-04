# Development Practices & Workflows

## Code Conventions

- Mimic existing code style, use existing libraries
- Check if library is used before importing (package.json, Cargo.toml)
- Never introduce code that exposes or logs secrets
- **NO COMMENTS unless asked** - code should be self-documenting
- **Never use `#[allow(dead_code)]`** - remove unused code instead (exception: dataflow module APIs)

## Rust Best Practices

### Derive Over Manual Implementation
```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]  // ✅ Prefer derives
struct DragState { active_divider: Option<DividerType>, position: (f32, f32) }
```
- Add `Copy` for small types to eliminate `.clone()` calls
- Use `Copy` for: enums with simple variants, small structs, primitives

### Modern Formatting & Rust 2024
```rust
println!("{my_var}");  // ✅ Inline variables (not println!("{}", my_var))
pub fn signal(&self) -> impl Signal<Item = T> + use<T>  // Rust 2024 capture bounds
```

### Expressive State Types
```rust
let canvas_width: Option<f32> = None;  // ✅ Clearly "not measured yet"
let canvas_width = 0.0;                 // ❌ Misleading - is this "no data" or zero?
```
Use `Option<T>`, custom enums (`NotMeasured | Ready { width }`) or `Result<Option<T>, E>`.

### Backend Concurrency (CRITICAL)

**See `.claude/extra/technical/backend-concurrency.md` for comprehensive patterns.**

Key rules:
- **Atomic lock acquisition**: Acquire ALL locks before modifying ANY store
- **Double-checked locking**: Re-check cache after upgrading to write lock
- **spawn_blocking**: Wrap CPU-intensive parsing in `tokio::task::spawn_blocking`
- **Poison recovery**: Use `match` + `into_inner()` instead of `.unwrap()` on locks

## Compilation Verification (CRITICAL)

Run `makers start` and check mzoon output directly for errors. Use browser MCP to verify app works.

**Never report success without verification.** Even 1 error = task incomplete.

## Critical Reactive Antipatterns

### Core Principle: Signals vs Streams

**This codebase uses SIGNALS, not STREAMS.**

| Signals | Streams |
|---------|---------|
| Broadcast - all observers get every value | Consumed - only one consumer gets each value |
| Use `.for_each_sync()` or `map_ref!` | Use only for mpsc receivers (backend connection) |
| `.to_stream()` almost always WRONG | Appropriate for genuine async message passing |

**If you see `.to_stream()` on a signal, it's almost always wrong.**

### 1. SignalVec → Signal Conversion (NEVER USE)
```rust
// ❌ Causes 20+ renders from single change
TRACKED_FILES.signal_vec_cloned().to_signal_cloned().map(|files| {...})

// ✅ Use items_signal_vec or dedicated Mutable<Vec<T>>
.items_signal_vec(TRACKED_FILES.signal_vec_cloned().map(|item| render(item)))
```

### 2. Static Signal Bypass
Static `OnceLock<Mutable<T>>` signals never update - connect to real domain events instead.

### 3. Actor.get() Doesn't Exist
No `.get()` method by design. Use: `.signal().to_stream().next().await` or `map_ref!`

### 4. UI Business Logic
```rust
// ❌ UI caching state
pub fn toggle_theme() { let current = theme_now(); /* toggle */ }
// ✅ UI sets input Mutable, domain observes
pub fn toggle_theme_requested() { app_config().inputs.theme_toggle.set(true); }
```

### 5. TreeView with child_signal (NEVER)
Use `.items_signal_vec()` always - `child_signal(map_ref!{...})` breaks TreeView rendering.

### 6. Zombie Actors
Recognition: `std::future::pending::<()>().await`, underscore params `|_handle|`
Fix: Connect proper event streams or delete.

### 7. Timer Workarounds
Never use `Timer::sleep()` for timing coordination. Use `Task::next_macro_tick().await` or signal waiting.

### 8. Task::start vs Task::start_droppable (CRITICAL)

**When TaskHandle is dropped, it calls `abort()` on the inner AbortHandle - the future IS CANCELLED!**

```rust
// ❌ CRITICAL BUG: Handle dropped immediately → Future aborted!
let _ = Task::start_droppable(async move { important_work().await; });

// ❌ STILL WRONG: Handle dropped at end of scope → Future aborted!
let _task = Task::start_droppable(async move { important_work().await; });
// _task dropped here when function/block ends!

// ✅ CORRECT: Fire-and-forget that MUST complete → Use Task::start()
Task::start(async move { important_work().await; });  // No handle, runs to completion

// ✅ CORRECT: Need cancellation → Store handle for lifetime needed
struct MyDomain {
    _background_task: TaskHandle,  // Lives with struct
}
let handle = Task::start_droppable(async move { ... });
self.handles.push(handle);  // Store in collection
```

**Rules:**
- **Fire-and-forget (must complete)**: Use `Task::start()` - no handle, cannot be cancelled
- **Cancellable work**: Use `Task::start_droppable()` AND store handle for needed lifetime
- **NEVER** use `let _ = Task::start_droppable(...)` - immediate abort!
- **NEVER** use `let _task = Task::start_droppable(...)` at function scope end - aborted when scope ends!

### 9. Signal-to-Stream Antipattern (CRITICAL)

**NEVER convert signals to streams for sync work:**

```rust
// ❌ ANTIPATTERN: Async machinery for sync work
Arc::new(Task::start_droppable(async move {
    let mut stream = signal.to_stream().fuse();
    while let Some(v) = stream.next().await {
        mutable.set_neq(v);  // This is sync!
    }
}))

// ✅ CORRECT: Native signal binding
Arc::new(Task::start_droppable(
    signal.for_each_sync(|v| mutable.set_neq(v))
))

// ✅ CORRECT: Combine multiple signals with map_ref!
Arc::new(Task::start_droppable(
    map_ref! {
        let a = signal_a,
        let b = signal_b => compute(a, b)
    }.for_each_sync(|result| state.set_neq(result))
))
```

**When `.to_stream()` IS appropriate:**
- `select!` macro with genuine async work (network I/O, timers, DOM timing)
- Backend message loops (unbounded MPSC receiver streams)
- Canvas rendering coordination with multiple async sources

**When `.to_stream()` is WRONG:**
- Endless loops that only call sync methods
- Reading one value via `.next().await` (signals emit initial value on subscribe)
- Combining signals that could use `map_ref!`
- Manual deduplication that `.dedupe_cloned()` handles

### 10. SignalVec to Signal Conversion (AVOID)

**Delay `.to_signal_cloned()` as long as possible - it clones the ENTIRE Vec on every change:**

```rust
// ❌ EXPENSIVE: Clones Vec<TrackedFile> (large structs) on every change
tracked_files.signal_vec_cloned()
    .to_signal_cloned()  // Full Vec clone here!
    .map(|files| compute_range(&files))

// ✅ CHEAPER: Map to small data first, then convert
tracked_files.signal_vec_cloned()
    .map(|file| (file.path.clone(), file.time_range()))  // Extract only needed data
    .to_signal_cloned()  // Now cloning Vec<(String, Option<Range>)> - much smaller
    .map(|ranges| aggregate_ranges(&ranges))

// ✅ BEST: Avoid .to_signal_cloned() entirely if possible
// Use items_signal_vec() in UI, or fold/reduce patterns
```

**Rules:**
- **Map before converting**: Extract only needed fields before `.to_signal_cloned()`
- **Avoid large struct clones**: `TrackedFile`, `WaveformFile` are expensive to clone
- **Question the need**: Do you really need Signal<Vec> or can you use SignalVec directly?

### 11. Dual Stream Consumers (CRITICAL BUG)

**Multiple tasks consuming same signal as stream = only ONE gets the value!**

```rust
// ❌ CRITICAL BUG: Two tasks, same signal as stream
let mut stream_a = flag.signal().to_stream().fuse();  // Task A
let mut stream_b = flag.signal().to_stream().fuse();  // Task B

// When flag changes: Task A gets value, Task B waits FOREVER!
```

**Fix:** Each observer should use its own signal subscription:
```rust
// ✅ CORRECT: Each task observes signal directly
flag.signal().for_each_sync(|v| { /* Task A logic */ });
flag.signal().for_each_sync(|v| { /* Task B logic */ });
```

### 12. Multiple Streams in select! (MISS UPDATES)

```rust
// ❌ Can miss updates when multiple signals change together
let mut stream_a = signal_a.to_stream().fuse();
let mut stream_b = signal_b.to_stream().fuse();
loop {
    select! {
        a = stream_a.next() => { ... }
        b = stream_b.next() => { ... }  // May miss if both change!
    }
}

// ✅ CORRECT: Combine signals first
map_ref! {
    let a = signal_a,
    let b = signal_b => (a.clone(), b.clone())
}.for_each_sync(|(a, b)| { ... })
```

### 13. Flag-Based Async Coordination (FRAGILE)

```rust
// ❌ FRAGILE: Mutable<bool> flags for async coordination
let config_loaded_flag = Mutable::new(false);
// ... somewhere else checks flag.get() - race conditions!

// ✅ BETTER: Use state enum or proper signal observation
enum ConfigState { NotLoaded, Restoring, Ready }
let config_state = Mutable::new(ConfigState::NotLoaded);
```

### 14. Missing .dedupe() (DUPLICATE WORK)

```rust
// ❌ Fires handler even when value unchanged
signal.for_each_sync(|v| expensive_work(v));

// ✅ CORRECT: Only fire when value actually changes
signal.dedupe_cloned().for_each_sync(|v| expensive_work(v));
```

## WASM Constraints (CRITICAL)

- **All I/O on backend** - WASM filesystem blocks main thread, freezes browser
- **Use `zoon::println!()`** - `std::println!()` does nothing
- **Never use `cargo build/check`** - only mzoon handles WASM

## Dev Server Management

- **Kill zombie processes**: `makers kill` - use this to kill stale/hung dev server processes
- **Run dev server**: `makers start` - output appears directly, watch for compilation errors
- Backend compilation takes DOZENS OF SECONDS TO MINUTES - wait for it

## Verification Requirements

- **NEVER claim success without browser MCP verification**
- If verification fails, tell user immediately with specific reason
- Check compilation before testing

## Refactoring Rules

1. Copy complete code to destination first
2. Verify compilation succeeds
3. Only then remove from source
4. NEVER create placeholder functions or empty stubs
5. **Never remove business logic** - convert to new architecture preserving functionality

## State Management

**Pure Reactive Dataflow** - State change IS the event.

Quick rules:
- `Mutable<T>` for state, `.signal()` for observation
- NO mpsc channels for state - use `Mutable<Option<T>>` instead
- External code sets Mutables, domains observe signals
- Domain-driven design: `TrackedFiles` not `FileManager`
- NO Manager/Service/Controller patterns

```rust
// ✅ CORRECT: Pure signal observation
inputs.some_request.signal()
    .for_each_sync(|maybe_req| {
        if let Some(req) = maybe_req {
            handle(req);
            inputs.some_request.set(None);  // Clear after handling
        }
    });

// ❌ WRONG: mpsc channels for state sync
let (sender, receiver) = mpsc::unbounded();
sender.unbounded_send(state);  // Don't do this!
```

## Dataflow API Protection

**Never modify dataflow module API without explicit confirmation.**

## Clarification Protocol

Ask before complex tasks:
- **Specificity**: "Entire extension styled, or just asterisks?"
- **Context**: "Full screen height with margin, or edge-to-edge?"
- **Scope**: "Match Files panel specifically?"

## File Organization

- **Never create generic files**: `utils.rs`, `helpers.rs`, `types.rs`
- Split by domain objects, not technical categories
- Default to `pub` fields unless specific reason for privacy
- Place utilities in their domain modules

## Debugging Patterns

- **Signal routing**: Trace from UI backwards - update the signal UI reads
- **"Loading..." stuck**: Fast ops appearing slow = broken reactivity
- **Duplicate calls**: Multiple handlers for same trigger - use mutually exclusive conditions
- **Config restoration timing**: Immediate sync + future changes pattern

## Work Ethics

- **Check existing code first** - often just need to connect working backend
- **No shortcuts** - either fix properly or be honest about limitations
- **No ugly hacks** - fix root cause, not symptoms
- **Quality over appearance** - partial correct > complete broken

## Quality Checklist

- [ ] Pure signal dataflow (no mpsc channels for state)
- [ ] External sets Mutables, domains observe signals
- [ ] No SignalVec→Signal antipatterns
- [ ] No `.to_stream()` on signals (use `for_each_sync` or `map_ref!`)
- [ ] Public field architecture maintained
- [ ] Compilation successful (0 errors)
- [ ] Browser MCP verification passed

## Antipattern Search Commands

Use these to audit the codebase:

```bash
# Find .to_stream() on signals (almost always wrong)
grep -rn "signal.*\.to_stream()" frontend/src/

# Find multiple .to_stream() in same file (race condition)
grep -n "\.to_stream()" frontend/src/**/*.rs | cut -d: -f1 | sort | uniq -c | sort -rn

# Find Mutable<bool> flags (fragile coordination)
grep -n "Mutable::new(false)\|Mutable::new(true)" frontend/src/**/*.rs

# Find for_each with async {} (should use for_each_sync)
grep -B2 "async {}" frontend/src/**/*.rs

# Find signal without dedupe before for_each
grep -n "signal_cloned()\.for_each" frontend/src/**/*.rs
```
