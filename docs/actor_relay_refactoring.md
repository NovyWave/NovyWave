# Actor+Relay Refactoring Guide

Comprehensive guide for refactoring from traditional MoonZone patterns to Actor+Relay architecture, including all hard-learned lessons and antipatterns to avoid.

## Table of Contents

1. [Implementation Roadmap](#implementation-roadmap)
2. [Refactoring Strategy](#refactoring-strategy)
3. [Modern Pattern Improvements](#modern-pattern-improvements)
4. [Critical Antipatterns](#critical-antipatterns)
5. [Step-by-Step Refactoring](#step-by-step-refactoring)
6. [Testing Patterns](#testing-patterns)
7. [Performance Considerations](#performance-considerations)
8. [Troubleshooting Guide](#troubleshooting-guide)
9. [Best Practices Summary](#best-practices-summary)

## Implementation Roadmap

### Phase 1: Core Infrastructure (Week 1)
- [ ] Implement `Relay<T>` with futures::channel
- [ ] Implement `Actor<T>`, `ActorVec<T>`, `ActorBTreeMap<K,V>`
- [ ] Add debug tracing and connection tracking
- [ ] Create unit tests for core functionality

### Phase 2: Local Component Development (Ongoing)
- [ ] Develop new components using local Actor+Relay patterns
- [ ] Create reusable component patterns and examples
- [ ] Build testing harness for local Actor components
- [ ] Document local state best practices

### Development Strategy
1. **Local-First Design**: Start with local Actor+Relay patterns for new components
2. **Component Isolation**: Keep state contained within component boundaries
3. **Testing**: Test each component independently
4. **Documentation**: Document reusable patterns for team use

### Success Metrics
- Zero recursive lock panics
- Reduced rendering (from 30+ to <5 per operation)
- All state mutations traceable
- Improved test coverage (>80%)
- Cleaner component boundaries

## Refactoring Strategy

### Migration Priority Order
1. **Start with SimpleState**: Replace local Mutable usage first
2. **Apply relay()**: Update Actor initialization patterns  
3. **Remove .get() calls**: Convert to signal-based access
4. **Local state refactor**: Move from global to local state architecture
5. **Testing conversion**: Signal-based testing throughout

### Incremental Approach
1. **Start with isolated components** (like Counter example)
2. **Keep it simple initially** - avoid premature optimization
3. **Test each refactoring step** before adding complexity
4. **Remove abstractions that don't add value**

### Common Migration Steps
1. **Identify the core state** (what was in the `Mutable`)
2. **Define minimal event types** (start with primitives)
3. **Create Actor with Default impl** (match original patterns)
4. **Use direct field access** (public Actor/Relay fields)
5. **Replace separate get/set with atomic update**
6. **Add tests in separate section**

## Modern Pattern Improvements

### 1. relay() vs clone! Macro

**❌ Old complex pattern with clone! macro**
```rust
let relay = Relay::new();
let actor = Actor::new(initial, clone!((relay) async move |state| {
    relay.subscribe().for_each(clone!((state) async move |event| {
        // Complex clone management, nested async blocks
    })).await;
}));
```

**✅ New streamlined pattern**
```rust
let (relay, stream) = relay();
let actor = Actor::new(initial, async move |state| {
    while let Some(event) = stream.next().await {
        // Direct access, clear control flow
    }
});
```

**Benefits**: 60% less boilerplate, easier debugging, cleaner error handling

### 2. SimpleState for Eliminating Mutable Usage

**❌ Inconsistent Mutable usage throughout codebase**
```rust
static DIALOG_OPEN: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
static LOADING: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
```

**✅ Unified SimpleState pattern (Actor+Relay internally)**
```rust
#[derive(Clone, Debug)]
struct SimpleState<T: Clone + Send + Sync + 'static> {
    pub value: Actor<T>,
    pub setter: Relay<T>,
}

impl<T: Clone + Send + Sync + 'static> SimpleState<T> {
    fn new(initial: T) -> Self {
        let (setter, mut setter_stream) = relay();
        
        let value = Actor::new(initial, async move |state| {
            while let Some(new_value) = setter_stream.next().await {
                state.set_neq(new_value);
            }
        });
        
        SimpleState { value, setter }
    }
    
    // Convenient methods
    fn set(&self, value: T) { self.setter.send(value); }
    fn signal(&self) -> impl Signal<Item = T> { self.value.signal() }
}

struct DialogState {
    is_open: SimpleState<bool>,
    is_loading: SimpleState<bool>,
}
```

### 3. Multi-Stream Processing with join!()

**❌ Multiple Task::start calls - harder to coordinate**
```rust
Task::start(async { stream1.for_each(...).await });
Task::start(async { stream2.for_each(...).await });
Task::start(async { stream3.for_each(...).await });
```

**✅ Coordinated multi-stream processing**
```rust
ActorVec::new(vec![], async move |state| {
    future::join!(
        async { while let Some(event) = stream1.next().await { /* process */ } },
        async { while let Some(event) = stream2.next().await { /* process */ } },
        async { while let Some(event) = stream3.next().await { /* process */ } }
    );
});
```

### 4. Type Unification Pattern

**❌ Duplicate Types: Different structs for identical functionality**
```rust
// ANTIPATTERN: Separate types for identical operations
#[derive(Clone)]
struct ColumnControl {
    count: Actor<usize>,
    increment: Relay,
    decrement: Relay,
}

#[derive(Clone)]
struct RowControl {
    count: Actor<usize>,  // Identical to ColumnControl
    increment: Relay,     // Identical to ColumnControl  
    decrement: Relay,     // Identical to ColumnControl
}
```

**✅ Unified Type: Single type for both use cases**
```rust
// CORRECT: Single type handles both dimensions
#[derive(Clone)]
struct GridDimensionControl {
    pub count: Actor<usize>,
    pub increment: Relay,  
    pub decrement: Relay,
}

impl Default for GridDimensionControl {
    fn default() -> Self {
        let (increment, mut increment_stream) = relay();
        let (decrement, mut decrement_stream) = relay();
        
        let count = Actor::new(5, async move |state| {
            loop {
                select! {
                    Some(()) = increment_stream.next() => {
                        state.update(|current| current + 1);
                    }
                    Some(()) = decrement_stream.next() => {
                        state.update(|current| current.saturating_sub(1).max(1));
                    }
                }
            }
        });
        
        GridDimensionControl { count, increment, decrement }
    }
}
```

**Benefits**: 50% less code, single source of truth, unified methods, better maintenance

## Critical Antipatterns

### 1. Using clone! Macro Instead of relay()

**❌ DEPRECATED: Complex clone! pattern - harder to debug**
```rust
let relay = Relay::new();
Actor::new(initial, clone!((relay) async move |state| {
    relay.subscribe().for_each(clone!((state) async move |event| {
        // Nested async blocks, complex clone management
    })).await;
}));
```

**✅ MODERN: Direct stream access - much cleaner**
```rust
let (relay, stream) = relay();
Actor::new(initial, async move |state| {
    while let Some(event) = stream.next().await {
        // Simple imperative loop, direct access
    }
});
```

### 2. Multiple relay.send() Calls from Same Source Location

**⚠️ CRITICAL CONSTRAINT**: A Relay can only be sent from ONE source location in your code.

**❌ WRONG: Will panic with "multiple source" error**
```rust
fn test() {
    relay.send(()); // First source location  
    relay.send(()); // PANIC: Multiple source locations detected
}
```

**✅ CORRECT: Different approaches for multiple sends**
```rust
// Option 1: Use Task::start for different call sites
for _ in 0..3 {
    Task::start(async { relay.send(()); });
}

// Option 2: Batch relay with count parameter
batch_relay.send(3); // Send count instead of multiple individual sends

// Option 3: futures::stream::select for multiple triggers
let send_trigger_stream = futures::stream::select(
    enter_pressed_stream,
    send_button_clicked_stream
);
```

### 3. Raw Mutable Instead of SimpleState

**❌ INCONSISTENT: Raw Mutable scattered throughout**
```rust
static DIALOG_STATE: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
static FILTER: Lazy<Mutable<String>> = lazy::default();
```

**✅ UNIFIED: SimpleState for consistent local state management**
```rust
struct UIState {
    dialog_open: SimpleState<bool>,
    filter_text: SimpleState<String>,
}
```

### 4. Task::start_droppable in Actor Setup

**❌ BROKEN: TaskHandle drops immediately, cancelling subscription**
```rust
Actor::new(0, |state| async move {
    Task::start_droppable(async move {
        relay.subscribe().for_each(...).await;
    }); // TaskHandle dropped here - subscription cancelled!
});
```

**✅ CORRECT: Await subscription directly or use create_with_stream()**
```rust
let (relay, stream) = relay();
Actor::new(0, async move |state| {
    while let Some(event) = stream.next().await {
        // Process events
    }
});
```

### 5. Separate Get/Set Operations (Race Conditions)

**❌ RACE CONDITION: Value can change between get() and set()**
```rust
let current = state.get();  // ❌ Actor<T> doesn't provide .get() to prevent this!
state.set_neq(current + amount);
```

**✅ ATOMIC: Single operation, no races possible**
```rust
state.update(|value| value + amount);
```

### 6. Helper Methods Instead of Direct Field Access

**❌ DEPRECATED: Wrapper methods that just forward calls**
```rust
impl GridManager {
    pub fn set_columns(&self, cols: usize) -> Result<(), RelayError> {
        self.grid_controls.set_columns.send(SetColumns(cols))  // Just forwarding
    }
    
    pub fn config_signal(&self) -> impl Signal<Item = GridConfig> {
        self.config.signal()  // Just forwarding
    }
}
```

**✅ MODERN: Direct field access - cleaner and more obvious**
```rust
struct GridManager {
    pub config: Actor<GridConfig>,
    pub set_columns: Relay<usize>,
    pub counter_action: Relay<CounterAction>,
}

// Usage is direct - no method indirection needed:
GRID.set_columns.send(5);
GRID.config.signal();
GRID.counter_action.send(CounterAction::ChangeBy(id, 1));
```

## Step-by-Step Refactoring

### Local State with Struct Methods (MOST IDIOMATIC - Recommended Default)

**✅ Use struct methods for most applications:**
```rust
#[derive(Clone, Default)]
struct CounterApp {
    // Flattened state - no unnecessary wrapper structs
    value: Actor<i32>,
    change_by: Relay<i32>,
    ui_state: SimpleState<bool>,
}

impl CounterApp {
    // Clean method structure - no parameter passing chaos
    fn root(&self) -> impl Element {
        Column::new()
            .item(self.counter_controls())  // self method call
            .item(self.status_panel())      // self method call
    }
    
    fn counter_controls(&self) -> impl Element {
        Row::new()
            .item(self.counter_button("-", -1))  // Passing primitives
            .item_signal(self.value.signal())
            .item(self.counter_button("+", 1))
    }
    
    fn counter_button(&self, label: &str, step: i32) -> impl Element {
        Button::new()
            .label(label)
            .on_press({
                let change_by = self.change_by.clone();  // Clean clone access
                move || change_by.send(step)
            })
    }
}

// Perfect lifetime handling - works because Actors are Arc internally
fn main() {
    start_app("app", || CounterApp::default().root());
}
```

### Simplification Patterns

#### 1. Remove Unnecessary Wrapper Types
```rust
// ❌ Over-abstraction: Wrapper for simple values
struct ChangeBy(i32);
struct SetColumns(usize);

// ✅ Direct and clear: Use primitive types or consolidated enums
pub change_by: Relay<i32>,
pub set_columns: Relay<usize>,

// OR for related operations:
pub enum GridAction {
    SetColumns(usize),
    SetRows(usize),
}
pub grid_action: Relay<GridAction>,
```

#### 2. Use Default Trait for Zero-Config Initialization
```rust
// ❌ Custom constructor when not needed
impl Counter {
    pub fn new() -> Self { ... }
}
static COUNTER: Lazy<Counter> = Lazy::new(|| Counter::new());

// ✅ Matches original patterns with lazy::default()
impl Default for Counter {
    fn default() -> Self { ... }
}
static COUNTER: Lazy<Counter> = lazy::default();
```

#### 3. Event Consolidation
```rust
// ❌ Event proliferation: Too many tiny event types
pub increment: Relay<IncrementCounter>,
pub decrement: Relay<DecrementCounter>,
pub reset: Relay<ResetCounter>,
pub set_value: Relay<SetCounterValue>,

// ✅ Consolidated enum: Related operations grouped
pub enum CounterAction {
    ChangeBy(String, i32),  // Handles both increment and decrement
    Reset(String),
    SetValue(String, i32),
}

// Single relay for all counter operations
pub counter_action: Relay<CounterAction>,
```

#### 4. Flatten Nested Relay Bundles
```rust
// ❌ Over-organization: Nested relay bundles add complexity
struct GridManager {
    grid_controls: GridControlRelays,    // Nested bundles
    counter_controls: CounterRelays,     // Add indirection
}

// Usage requires deep navigation:
manager.grid_controls.set_columns.send(SetColumns(5));

// ✅ Flat structure: Direct access, clear ownership
struct GridManager {
    // State
    pub config: Actor<GridConfig>,
    pub counters: ActorVec<CounterData>,
    
    // Events - all at same level for direct access
    pub set_columns: Relay<usize>,
    pub set_rows: Relay<usize>, 
    pub counter_action: Relay<CounterAction>,
}

// Usage is direct and obvious:
manager.set_columns.send(5);
manager.counter_action.send(CounterAction::Increment(id, 1));
```

### Atomic Operation Patterns

**⚠️ CRITICAL: Actor<T> intentionally does NOT provide a .get() method to prevent race conditions.**

**✅ CORRECT: Atomic operations using state.update() inside Actor**
```rust
let (change_by, mut change_stream) = relay();
let value = Actor::new(0, async move |state| {
    while let Some(amount) = change_stream.next().await {
        // This is atomic - no race conditions possible
        state.update(|current| current + amount);
    }
});
```

**✅ CORRECT: Separate atomic operations for common patterns**
```rust
let (increment, mut increment_stream) = relay();
let (decrement, mut decrement_stream) = relay();
let value = Actor::new(0, async move |state| {
    loop {
        select! {
            Some(()) = increment_stream.next() => {
                state.update(|current| current + 1);  // Atomic increment
            }
            Some(()) = decrement_stream.next() => {
                state.update(|current| current.saturating_sub(1)); // Atomic decrement
            }
        }
    }
});
```

### Safe Arithmetic Patterns

**❌ Type Conversion Antipattern:**
```rust
// BAD: Type conversions everywhere create boilerplate and potential bugs
let count = count_actor.signal().map(|c| c as i32);  // usize -> i32
let new_value = (current as i32) - 1;                // More conversions
let final_value = new_value.max(1) as usize;         // Back to usize
```

**✅ Safe Standard Library Math:**
```rust
// GOOD: Pure usize arithmetic with safe operations
state.update(|current| {
    current.saturating_sub(1).max(1)  // Never goes below 1, no conversions
});

// GOOD: Clean increment without conversions
state.update(|current| current + 1);  // Simple, pure usize

// GOOD: Safe operations handle edge cases
state.update(|current| {
    current.saturating_add(delta)     // Won't overflow
        .min(MAX_ALLOWED_VALUE)       // Cap at maximum
});
```

**Common Safe Patterns:**
```rust
// Decrement with minimum bound
current.saturating_sub(amount).max(minimum)

// Increment with maximum bound  
current.saturating_add(amount).min(maximum)

// Safe range operations
current.saturating_sub(delta).clamp(min_value, max_value)

// Checked operations for critical paths
current.checked_add(amount).unwrap_or(current)
```

## Testing Patterns

### Essential Testing Protocol

All examples consistently use this pattern - **Signal-based reactive testing**:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_counter_increment() {
        let counter = Counter::default();
        
        // Send event through relay
        counter.change_by.send(3);
        
        // ✅ CORRECT: Use signal.await to get value reactively
        let final_value = counter.value.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, 3);
    }
    
    #[async_test]  
    async fn test_multiple_operations() {
        let counter = Counter::default();
        
        // Multiple operations
        counter.change_by.send(5);
        counter.change_by.send(-2);
        counter.change_by.send(1);
        
        // ✅ CORRECT: Await signal - no Timer::sleep needed!
        let final_value = counter.value.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, 4);  // 0 + 5 - 2 + 1 = 4
    }
    
    #[async_test]
    async fn test_safe_arithmetic() {
        let control = GridDimensionControl::default();  // Starts at 5
        
        // Test saturating subtraction
        for _ in 0..10 {
            control.decrement.send(());
        }
        
        // ✅ CORRECT: Use signal reactively - no .get() needed
        let final_count = control.count.signal().to_stream().next().await.unwrap();
        assert_eq!(final_count, 1);  // Should never go below 1
    }
}
```

### Why signal().next().await Works

**The Reactive Testing Model:**
- Relay sends are **synchronous** (immediate return) 
- Actor processing is **asynchronous** (happens later)
- `signal().next().await` **waits** for the next signal emission
- This naturally waits for Actor processing to complete

**Testing Timeline:**
```
1. relay.send() → Returns immediately
2. Actor receives event → Happens on next tick  
3. state.update() → Happens inside Actor
4. signal().next().await → Waits for signal from step 3
5. Test continues with correct value
```

**Benefits:**
- **No arbitrary timeouts**: Wait exactly as long as needed
- **More reliable**: No race conditions from insufficient wait times
- **Consistent with architecture**: Uses reactive patterns throughout
- **Faster tests**: No unnecessary delays

### Proper Test Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // ✅ Correct reactive pattern - no .get() or Timer::sleep()
    #[async_test]
    async fn test_pattern_name() {
        // 1. Create instance using Default
        let app = CounterApp::default();
        
        // 2. Send events through relays
        app.increment.send(());
        
        // 3. Await signal reactively - no arbitrary timeouts
        let result = app.value.signal().to_stream().next().await.unwrap();
        
        // 4. Assert final state
        assert_eq!(result, expected_value);
    }
}
```

## Performance Considerations

### Vec Indices vs String IDs

**✅ Use Vec indices for performance-critical operations:**
```rust
// GOOD: Vec index access - O(1) performance
struct CounterGrid {
    values: ActorVec<i32>,
    change: Relay<(usize, i32)>,  // Index + amount
}

// Usage: Direct index access
counters.change.send((index, -1));  // Decrement counter at index 5
```

**❌ Avoid string IDs for frequent operations:**
```rust
// BAD: String ID lookup - O(n) search performance
struct CounterGrid {
    values: ActorBTreeMap<String, i32>,  // String lookup overhead
    change: Relay<(String, i32)>,        // ID + amount
}
```

### Memory Efficiency Patterns

**✅ Type aliases for frequently cloned data:**
```rust
// Reduces clone overhead for frequently passed data
type Username = Arc<String>;      // Instead of String
type MessageText = Arc<String>;   // Instead of String  
type FilePath = Arc<PathBuf>;     // Instead of PathBuf

// Usage - cheaper clones
pub username_changed: Relay<Username>,  // Arc clone instead of String clone
```

**✅ Efficient state updates:**
```rust
// set_neq only triggers signals when value actually changes
state.set_neq(new_value);  // No signal if value is same

// saturating operations prevent overflow allocations
current.saturating_add(amount).min(MAX_SIZE)
```

### Multi-Stream Performance

**✅ Concurrent stream processing:**
```rust
// GOOD: True concurrent processing with join!()
ActorVec::new(vec![], async move |state| {
    future::join!(
        async { /* Process stream 1 */ },
        async { /* Process stream 2 */ },
        async { /* Process stream 3 */ },
    );
});
```

**❌ Sequential Task::start overhead:**
```rust
// BAD: Multiple separate tasks - coordination overhead
Task::start(async { stream1.for_each(...).await });
Task::start(async { stream2.for_each(...).await });  
Task::start(async { stream3.for_each(...).await });
```

### Signal Efficiency

**✅ Minimize signal chain depth:**
```rust
// GOOD: Direct signal access
COUNTER.value.signal()

// GOOD: Single map operation  
COUNTER.value.signal().map(|v| format!("{}", v))
```

**❌ Avoid excessive signal chaining:**
```rust
// BAD: Deep signal chains cause recomputation cascades
COUNTER.value.signal()
    .map(|v| v * 2)
    .map(|v| v + 1)
    .map(|v| format!("{}", v))
    .map(|s| s.to_uppercase())
```

## Troubleshooting Guide

### Common Issues & Solutions

#### 1. Recursive Lock Panics
**Symptoms:** `RuntimeError: unreachable at std::sys::sync::rwlock::no_threads::RwLock::write`

**Causes:**
- Multiple `Task::start` calls processing same state concurrently
- Using `for_each_sync` handlers that send messages while locks are held
- Missing `Task::next_macro_tick().await` in message processing loops

**Solutions:**
```rust
// ✅ Use async signal handlers to defer execution
COLLECTION.signal_vec_cloned().for_each(|data| async move {
    send_message_that_modifies_collection(data); // Safe through actor model
}).await;

// ✅ Add event loop yielding in sequential processing
for message in messages {
    Task::next_macro_tick().await;  // ESSENTIAL: Yield to event loop
    process_message(message).await;  // Sequential processing
}
```

#### 2. Multiple Source Location Errors
**Symptoms:** Panic with "multiple source locations detected"

**Cause:** Calling `relay.send()` from multiple locations in code

**Solutions:**
```rust
// ✅ Option 1: Use Task::start for different call sites
for _ in 0..3 {
    Task::start(async { relay.send(()); });
}

// ✅ Option 2: Use stream merging
let combined_stream = futures::stream::select(
    button_clicked_stream,
    keyboard_enter_stream
);
```

#### 3. Actor Not Receiving Events
**Symptoms:** Events sent but Actor never processes them

**Causes:**
- Stream not properly consumed in Actor
- TaskHandle dropped too early
- Actor task panicked and exited

**Solutions:**
```rust
// ✅ Ensure proper stream consumption
let (relay, mut stream) = relay();
let actor = Actor::new(initial, async move |state| {
    while let Some(event) = stream.next().await {  // Must consume stream
        // Process events
    }
});

// ✅ Check for panics in Actor task
let actor = Actor::new(initial, async move |state| {
    while let Some(event) = stream.next().await {
        // Add error handling to prevent panics
        if let Err(e) = process_event(event).await {
            zoon::println!("Actor error: {:?}", e);
            continue;  // Don't exit loop on errors
        }
    }
});
```

#### 4. Over-rendering Issues
**Symptoms:** UI updates excessively, performance problems

**Causes:**
- Signal cascades causing multiple renders
- Missing `.dedupe_cloned()` on signal chains
- SignalVec → Signal conversion instability

**Solutions:**
```rust
// ✅ Add deduplication to signal chains
TRACKED_FILES.signal_vec_cloned()
    .to_signal_cloned()
    .dedupe_cloned()  // Prevent duplicate emissions
    .map(|files| render_files(&files))

// ✅ Use items_signal_vec for collections instead
.items_signal_vec(TRACKED_FILES.signal_vec_cloned().map(|item| render_item(item)))
```

#### 5. Testing Timing Issues
**Symptoms:** Tests fail intermittently, values not updated

**Cause:** Using arbitrary timeouts instead of signal-based waiting

**Solution:**
```rust
// ❌ Wrong: Arbitrary timeouts
counter.increment.send(());
Timer::sleep(100).await;  // Unreliable
let value = counter.value.get();  // .get() doesn't exist anyway

// ✅ Correct: Signal-based waiting
counter.increment.send(());
let value = counter.value.signal().to_stream().next().await.unwrap();
assert_eq!(value, 1);
```

### Debugging Tools

#### Debug Tracing
```rust
#[cfg(debug_assertions)]
macro_rules! actor_trace {
    ($actor:expr, $msg:expr) => {
        if ActorDebugger::is_enabled() {
            zoon::println!("[{}] {}", $actor.name(), $msg);
        }
    };
}

// Usage in Actor
actor_trace!(self, format!("Processing event: {:?}", event));
```

#### Testing Utilities
```rust
/// Mock relay for testing
pub struct MockRelay<T> {
    events: Arc<Mutex<Vec<T>>>,
    auto_respond: Option<Box<dyn Fn(&T) -> Option<T>>>,
}

impl<T: Clone> MockRelay<T> {
    pub fn new() -> Self;
    pub fn events(&self) -> Vec<T>;  // Get all emitted events
    pub fn simulate_emit(&self, event: T);  // Simulate emission
}
```

### Multi-Stream Decision Guide
```
Multiple streams needed?
├── Shared state/coordination? → Use select!()
├── Independent processing? → Use join!()
├── Same event type, multiple sources? → Use futures::stream::select()
└── Single stream? → Use while let
```

## Best Practices Summary

### Architecture Fundamentals
- **All state through Actors**: No raw `Mutable<T>` usage - use `SimpleState` helper for local UI state
- **Signal-only state access**: No `.get()` methods - all state access through `.signal()` and signal-based testing
- **Atomic operations**: Use `state.update()` for modifications, never get-modify-set patterns
- **Local state by default**: Use struct methods with local state, global only when truly needed

### Modern Implementation Patterns  
- **relay()**: Always use instead of clone! macro patterns for cleaner Actor initialization
- **Imperative stream processing**: Use `while let Some(event) = stream.next().await` instead of `.for_each()` 
- **SimpleState consistency**: Use Actor+Relay internally for all local UI state (button hover, dialog open/closed)
- **Unified types**: Single type for similar operations (avoid ColumnControl vs RowControl duplication)

### Error Handling Strategy
- **UI events**: Use `.send()` (auto-logged errors)
- **Critical operations**: Use `.try_send()` (explicit error handling)
- **Keep it simple**: Don't over-engineer - basic error states and retry patterns

### Type Safety & Performance
- **Saturating arithmetic**: Always use `.saturating_add()`, `.saturating_sub().max(1)` for safe math
- **Type unification**: Pure `usize` for counts, avoid unnecessary `i32`/`usize` conversions  
- **Vec indices**: Use index-based operations for performance instead of string ID lookups
- **Direct field access**: Make Actor/Relay fields public - they're inherently safe

### Anti-Patterns to Avoid
- ❌ Raw `Mutable<T>` usage anywhere in Actor+Relay code
- ❌ clone! macro patterns - use `create_with_stream()` instead  
- ❌ `.get()` method usage - signal-based access only
- ❌ Complex *Manager/*Service patterns - keep business logic simple
- ❌ Multiple Task::start calls - use `join!()` for coordination
- ❌ Timer-based testing - use signal streams for deterministic tests

### Naming Conventions

#### Component Naming Patterns
```rust
struct ChatApp {
    // State: *_actor for single values, without suffix for collections
    username_actor: Actor<String>,
    message_text_actor: Actor<String>, 
    messages_actor: ActorVec<Message>,        // Collection actors
    
    // Events: *_relay for all relay types
    enter_pressed_relay: Relay,               // Unit relays (Relay<()>)
    username_input_changed_relay: Relay<Username>,  // Typed relays
    
    // External integrations: *_adapter for service bridges
    connection_adapter: ConnectionAdapter<UpMsg, DownMsg>,
}
```

#### Descriptive Relay Names
```rust
// ✅ Descriptive names indicating purpose and type
pub file_added: Relay<PathBuf>,           // Event-based: what happened
pub text_changed: Relay<String>,          // Change-based: what property changed
pub button_clicked: Relay,                // Action-based: user interaction
pub zoom_level_changed: Relay<f64>,       // Property change with value
pub selection_cleared: Relay,             // State change notification
```

## Conclusion

The Actor/Relay architecture provides **traceability**, **controlled mutations**, **no recursive locks**, and **better testability** while maintaining Zoon's reactive programming model. The key insights from pattern evolution:

1. **Imperative > Functional**: `while let Some(event) = stream.next().await` is cleaner and more debuggable than nested `.for_each()` closures
2. **Direct Stream Access**: `create_with_stream()` eliminates 60% of boilerplate compared to clone! macro patterns
3. **Unified State Management**: SimpleState provides consistency for local UI state, Actor for shared state
4. **Multi-Stream Coordination**: `join!()` enables true concurrent stream processing vs independent Task::start calls

Start simple, stay atomic, and let the architecture's benefits emerge naturally. The migration can be done incrementally, with each migrated component becoming more testable and maintainable, providing immediate value even before the full migration is complete.