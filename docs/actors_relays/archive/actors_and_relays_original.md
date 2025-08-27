# Actors and Relays Architecture

This document focuses on **local state patterns** using Actor+Relay architecture. For global state patterns and migration strategies, see [actors_and_relays_globals.md](actors_and_relays_globals.md).

## Table of Contents
1. [Motivation & Problem Analysis](#motivation--problem-analysis)
2. [Module Structure](#module-structure)
3. [Architecture Overview](#architecture-overview)
4. [API Design](#api-design)
5. [Type Safety](#type-safety)
6. [Migration Patterns](#migration-patterns)
7. [Implementation Examples](#implementation-examples)
8. [Testing & Debugging](#testing--debugging)
9. [Implementation Roadmap](#implementation-roadmap)

## Motivation & Problem Analysis

### Current Problems in NovyWave

The application currently suffers from several architectural issues stemming from uncontrolled global state mutations:

#### 1. **Unclear Mutation Sources** (69+ Global Mutables)
```rust
// Current: Who modifies TRACKED_FILES? Multiple places!
pub static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = lazy::default();

// Found 141 lock_mut() calls across 12 files - impossible to trace
TRACKED_FILES.lock_mut().push_cloned(file);  // views.rs:333
TRACKED_FILES.lock_mut().retain(|f| ...);     // state.rs:214
TRACKED_FILES.lock_mut().set_cloned(i, ...);  // config/triggers.rs:84
```

#### 2. **Recursive Lock Panics**
```rust
// Current problematic pattern causing panics:
TRACKED_FILES.signal_vec_cloned().for_each_sync(|files| {
    // This runs while parent lock is still held!
    TRACKED_FILES.lock_mut().update();  // PANIC: Recursive lock!
});
```

#### 3. **Over-Rendering Issues**
```rust
// Signal cascade causing 30+ renders in 300ms:
TRACKED_FILES → SMART_LABELS → child_signal(map_ref!) → Full TreeView Recreation

// Console spam during file loading:
🔨 [TreeView] RENDERING tree item: file_1
🔨 [TreeView] RENDERING tree item: file_1  // Same item, 30+ times!
```

#### 4. **No Traceability**
- Can't debug where mutations come from
- No way to log/track state changes systematically
- Difficult to test - global state everywhere
- Race conditions from multiple concurrent mutations

### Why Actor Model?

The Actor model provides:
- **Single point of mutation** - All changes go through one processor
- **Traceability** - Can log every state change with source
- **No recursive locks** - Sequential message processing
- **Testability** - Actors can be tested in isolation
- **Clear data flow** - Explicit message passing

## Module Structure

### Standalone, Extractable Design

The Actor/Relay system is designed as a **standalone module** that can be:
- Extracted as an independent library (`zoon-actors`)
- Moved directly to Zoon when battle-tested
- Used by any Zoon application

```rust
// frontend/src/reactive_actors/mod.rs
// OR as a separate crate: zoon-actors

pub mod relay;
pub mod actor;
pub mod actor_vec;
pub mod actor_btree_map;
pub mod testing;

// Re-export main types
pub use relay::Relay;
pub use actor::Actor;
pub use actor_vec::ActorVec;
pub use actor_btree_map::ActorBTreeMap;

// Testing utilities
pub use testing::{MockRelay, TestActor, ActorTestHarness};

// Dependencies (no app-specific imports!)
// - zoon (for Mutable, MutableVec, Task, etc.)
// - futures (via zoon re-exports)
// - std collections
```

### Key Design Principles

1. **Zero NovyWave Dependencies** - Pure Zoon/Rust implementation
2. **Minimal External Dependencies** - Only zoon and std
3. **WASM-First Design** - All async patterns are WASM-compatible
4. **Testing Built-in** - Mock implementations and test utilities included

## ⚠️ CRITICAL RULE: NO RAW MUTABLES ⚠️

**NEVER use raw `Mutable<T>` directly in Actor+Relay architecture!**

The entire purpose of this architecture is to eliminate uncontrolled state mutations from the 69+ global Mutables that caused recursive locks and over-rendering issues. **ALL state must be managed through Actors.**

### ❌ NEVER DO THIS:
```rust
// VIOLATION: Raw Mutable usage defeats the entire architecture
let username = Mutable::new("John");
let count = Mutable::new(0);

// This brings back all the original problems:
// - Multiple mutation points
// - No traceability  
// - Recursive lock potential
// - No controlled state flow
```

### ✅ ALWAYS USE ACTORS:
```rust
// CORRECT: All state managed through Actors
let username = Actor::new("John", async move |state| {
    while let Some(new_name) = name_changes.next().await {
        state.set_neq(new_name);  // Controlled mutation point
    }
});

let count = Actor::new(0, async move |state| {
    while let Some(delta) = count_changes.next().await {
        state.update(|current| current + delta);  // Traceable mutation
    }
});
```

### ONLY EXCEPTION: SimpleState Helper
The `SimpleState` helper is acceptable for truly local UI state (button hover, dropdown open/closed) as it's still a controlled abstraction:

```rust
// ACCEPTABLE: SimpleState helper for local UI only
let is_hovered = SimpleState::new(false);
// This is still controlled - wraps Mutable with clean API
```

#### Complete SimpleState Implementation (from Chat Example)

Based on the chat example, here's the correct implementation using Actor+Relay internally:

```rust
/// Unified helper for local UI state - uses Actor+Relay architecture internally
#[derive(Clone, Debug)]
pub struct SimpleState<T: Clone + Send + Sync + 'static> {
    pub value: Actor<T>,
    pub setter: Relay<T>,
}

impl<T: Clone + Send + Sync + 'static> SimpleState<T> {
    pub fn new(initial: T) -> Self {
        let (setter, mut setter_stream) = relay();
        
        let value = Actor::new(initial, async move |state| {
            while let Some(new_value) = setter_stream.next().await {
                state.set_neq(new_value);
            }
        });
        
        SimpleState { value, setter }
    }
    
    // Convenient methods that delegate to Actor+Relay
    pub fn set(&self, value: T) {
        self.setter.send(value);
    }
    
    pub fn signal(&self) -> impl Signal<Item = T> {
        self.value.signal()
    }
}

// Usage pattern: Replace all global Mutables with local SimpleState
struct UIState {
    is_dialog_open: SimpleState<bool>,
    filter_text: SimpleState<String>,
    selected_index: SimpleState<Option<usize>>,
    hover_state: SimpleState<bool>,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            is_dialog_open: SimpleState::new(false),
            filter_text: SimpleState::new(String::new()),
            selected_index: SimpleState::new(None),
            hover_state: SimpleState::new(false),
        }
    }
}

// Clean usage throughout components:
fn dialog_button(ui_state: &UIState) -> impl Element {
    Button::new()
        .label_signal(ui_state.is_dialog_open.signal().map(|is_open| {
            if is_open { "Close Dialog" } else { "Open Dialog" }
        }))
        .on_press({
            let setter = ui_state.is_dialog_open.setter.clone();
            move || setter.send(true)
        })
}
```

**Key Benefits:**
- **True Actor+Relay Architecture**: Uses Actor+Relay internally, no raw Mutable violations
- **Consistent API**: Same patterns as full Actor usage (`.setter.send()` and `.value.signal()`)
- **Efficiency**: Only triggers signals when value actually changes
- **Type Safety**: Compile-time checking for all local UI state
- **Traceability**: All mutations go through relay system like other Actors
- **Testable**: Can be instantiated and tested in isolation using signal-based testing

**Rule**: When in doubt, use an Actor. The architecture benefits (traceability, controlled mutations, no recursive locks) only exist when ALL state goes through Actors.

## Architecture Overview

### Core Concepts

#### **Relay**: Type-safe Event Streaming
- Replaces lossy Signals with non-lossy Streams
- Typed messages ensure compile-time safety
- Multiple subscribers can listen to events
- Drops events when no listeners (efficiency)

#### **Actor**: Reactive State Management
- Owns a `Mutable<T>` and controls all mutations
- Processes events from Relays sequentially
- Provides reactive signals for UI binding
- Built-in debug tracing and connection tracking

### Architecture Diagram

```
UI Components
    ↓ (emit typed events)
Relays<T>
    ↓ (stream events)
Actors
    ↓ (update state)
Mutable State
    ↓ (signal changes)
UI Updates
```

## API Design

### Relay<T>

```rust
use std::borrow::Cow;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Type-safe event streaming for UI → Actor communication
#[derive(Clone, Debug, Default)]
pub struct Relay<T = ()> 
where 
    T: Clone + Send + Sync + 'static 
{
    sender: UnboundedSender<T>,
    has_subscribers: Arc<AtomicBool>,
    #[cfg(debug_assertions)]
    emit_location: Arc<Mutex<Option<&'static std::panic::Location<'static>>>>,
}

impl<T> Relay<T> 
where 
    T: Clone + Send + Sync + 'static
{
    /// Create a new relay
    pub fn new() -> Self;
    
    /// Send a value through the relay (dropped if no subscribers)
    /// Automatically logs any errors to browser console. For explicit error handling, use try_send().
    #[track_caller]
    pub fn send(&self, value: T);
    
    /// Try to send a value through the relay, returning Result for explicit error handling
    /// Use this when you need to handle send failures programmatically
    #[track_caller] 
    pub fn try_send(&self, value: T) -> Result<(), RelayError>;
    
    /// Subscribe to receive values as a Stream
    pub fn subscribe(&self) -> impl Stream<Item = T>;
    
    /// Check if there are active subscribers (internal use)
    pub(crate) fn has_subscribers(&self) -> bool;
    
}

/// Creates a new Relay with an associated stream, following Rust's channel pattern.
/// 
/// This is the idiomatic way to create a Relay for use with Actors, eliminating
/// clone! macro boilerplate and providing direct stream access.
/// 
/// # Example
/// ```rust
/// // Just like Rust channels:
/// let (tx, rx) = channel();
/// 
/// // Actor+Relay pattern:
/// let (increment, mut increment_stream) = relay();
/// let (decrement, mut decrement_stream) = relay();
/// 
/// let counter = Actor::new(0, async move |state| {
///     loop {
///         select! {
///             Some(()) = increment_stream.next() => {
///                 state.update(|n| n + 1);
///             }
///             Some(()) = decrement_stream.next() => {
///                 state.update(|n| n.saturating_sub(1));
///             }
///         }
///     }
/// });
/// ```
pub fn relay<T>() -> (Relay<T>, impl Stream<Item = T>) {
    let relay = Relay::new();
    let stream = relay.subscribe();
    (relay, stream)
}


#[derive(Debug, Clone, thiserror::Error)]
pub enum RelayError {
    #[error("No subscribers for relay")]
    NoSubscribers,
    #[error("Channel closed")]
    ChannelClosed,
    #[cfg(debug_assertions)]
    #[error("Relay send called from multiple locations: previous {previous}, current {current}")]
    MultipleEmitters {
        previous: &'static std::panic::Location<'static>,
        current: &'static std::panic::Location<'static>,
    },
}
```

### Actor<T>

```rust
/// Single-value reactive state with controlled mutations
#[derive(Clone, Debug)]
pub struct Actor<T> 
where
    T: Clone + Send + Sync + 'static
{
    state: Mutable<T>,
    task: Arc<TaskHandle>,
    #[cfg(debug_assertions)]
    creation_location: &'static std::panic::Location<'static>,
}

impl<T> Actor<T>
where
    T: Clone + Send + Sync + 'static
{
    /// Create actor with initial state and async setup function
    #[track_caller]
    pub fn new<F>(initial: T, setup: F) -> Self 
    where
        F: for<'a> FnOnce(&'a Mutable<T>) -> impl Future<Output = ()> + 'a;
    
    /// Get reactive signal
    pub fn signal(&self) -> impl Signal<Item = T>;
    
    /// Get reference signal
    pub fn signal_ref<U>(&self, f: impl Fn(&T) -> U) -> impl Signal<Item = U>;
    
    // No .get() method - all state access must be through signals
    // This prevents race conditions and maintains architectural consistency
    // For testing, use signal-based assertions: signal().to_stream().next().await
}

```

### ActorVec<T>

```rust
/// Collection state management
#[derive(Clone, Debug)]
pub struct ActorVec<T> 
where
    T: Clone + Send + Sync + 'static
{
    items: MutableVec<T>,
    task: Arc<TaskHandle>,
    #[cfg(debug_assertions)]
    creation_location: &'static std::panic::Location<'static>,
}

impl<T> ActorVec<T>
where
    T: Clone + Send + Sync + 'static
{
    /// Create with initial items and async setup
    #[track_caller]
    pub fn new<F>(initial: Vec<T>, setup: F) -> Self
    where
        F: for<'a> FnOnce(&'a MutableVec<T>) -> impl Future<Output = ()> + 'a;
    
    /// Get reactive signal vec
    pub fn signal_vec(&self) -> impl SignalVec<Item = T>;
    
    /// Get length signal
    pub fn len_signal(&self) -> impl Signal<Item = usize>;
}
```

### ActorBTreeMap<K, V>

```rust
/// Ordered map state management
#[derive(Clone, Debug)]
pub struct ActorBTreeMap<K, V> 
where 
    K: Ord + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static
{
    map: MutableBTreeMap<K, V>,
    task: Arc<TaskHandle>,
    #[cfg(debug_assertions)]
    creation_location: &'static std::panic::Location<'static>,
}

impl<K, V> ActorBTreeMap<K, V>
where 
    K: Ord + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static
{
    /// Create with initial map and async setup
    #[track_caller]
    pub fn new<F>(initial: BTreeMap<K, V>, setup: F) -> Self
    where
        F: for<'a> FnOnce(&'a MutableBTreeMap<K, V>) -> impl Future<Output = ()> + 'a;
    
    /// Get reference signal for efficient map access
    pub fn signal_map_ref<U>(&self, f: impl Fn(&BTreeMap<K, V>) -> U) -> impl Signal<Item = U>;
    
    /// Get signal for specific key
    pub fn signal_for_key(&self, key: &K) -> impl Signal<Item = Option<V>>;
    
    /// Get keys as efficient SignalVec (only sends diffs)
    pub fn keys_signal_vec(&self) -> impl SignalVec<Item = K>;
    
    /// Get values as efficient SignalVec (only sends diffs)  
    pub fn values_signal_vec(&self) -> impl SignalVec<Item = V>;
    
    /// Get length signal
    pub fn len_signal(&self) -> impl Signal<Item = usize>;
    
    /// Get current value for key
    pub fn get(&self, key: &K) -> Option<V>;
}
```

## Modern Actor+Relay Patterns

Based on practical implementation experience, these patterns represent the most effective approaches discovered through refactoring real-world MoonZoon code.

### Core Pattern Evolution

#### From Functional to Imperative Stream Processing

**❌ Old Complex Pattern:**
```rust
// Overly complex with clone! macros and nested async blocks
let relay = Relay::new();
let actor = Actor::new(initial_state, clone!((relay) async move |state| {
    relay.subscribe().for_each(clone!((state) async move |event| {
        // Complex clone! management
        // Harder to debug
        // More boilerplate
    })).await;
}));
```

**✅ New Imperative Pattern:**
```rust
// Clean and simple with create_with_stream()
let (relay, stream) = relay();
let actor = Actor::new(initial_state, async move |state| {
    // Simple imperative loop - easier to debug and maintain
    while let Some(event) = stream.next().await {
        // Direct access to state and relay
        // Clear control flow
        // Less boilerplate
    }
});
```

#### Eliminating All Mutable Usage with SimpleState

**❌ Old Pattern: Raw Mutable everywhere**
```rust
// Local UI state scattered throughout components
static DIALOG_OPEN: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
static FILTER_TEXT: Lazy<Mutable<String>> = Lazy::new(|| Mutable::new(String::new()));
static SELECTED_INDEX: Lazy<Mutable<Option<usize>>> = Lazy::new(|| Mutable::new(None));

// Usage is inconsistent and error-prone
DIALOG_OPEN.set_neq(true);
let text = FILTER_TEXT.get();
```

**✅ New Pattern: SimpleState helper (Actor+Relay internally)**
```rust
/// Unified helper for local UI state - uses Actor+Relay internally
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
    
    // Convenient methods that delegate to Actor+Relay
    fn set(&self, value: T) { self.setter.send(value); }
    fn signal(&self) -> impl Signal<Item = T> { self.value.signal() }
}

// Consistent usage throughout application
struct DialogState {
    is_open: SimpleState<bool>,
    filter_text: SimpleState<String>,
    selected_index: SimpleState<Option<usize>>,
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            is_open: SimpleState::new(false),
            filter_text: SimpleState::new(String::new()),
            selected_index: SimpleState::new(None),
        }
    }
}
```

#### Multi-Stream Actor Pattern with join!()

**Advanced Pattern: Processing Multiple Streams Concurrently**
```rust
use futures::future;

struct MultiStreamProcessor {
    pub data_events: Relay<DataEvent>,
    pub config_events: Relay<ConfigChange>,
    pub timer_events: Relay<TimerTick>,
    pub results: ActorVec<ProcessedResult>,
}

impl MultiStreamProcessor {
    pub fn new() -> Self {
        // Create all streams at once
        let (data_events, data_stream) = relay();
        let (config_events, config_stream) = relay();
        let (timer_events, timer_stream) = relay();
        
        let results = ActorVec::new(vec![], async move |results_vec| {
            // Use join!() to process multiple streams concurrently
            future::join!(
                // Stream 1: Data processing
                async {
                    while let Some(event) = data_stream.next().await {
                        // Process data events
                        match event {
                            DataEvent::NewItem(item) => {
                                results_vec.lock_mut().push_cloned(ProcessedResult::from(item));
                            }
                            DataEvent::Clear => {
                                results_vec.lock_mut().clear();
                            }
                        }
                    }
                },
                
                // Stream 2: Configuration changes
                async {
                    while let Some(config) = config_stream.next().await {
                        // Apply configuration changes
                        zoon::println!("🔧 Config updated: {:?}", config);
                        // Update processing behavior based on config
                    }
                },
                
                // Stream 3: Timer events
                async {
                    while let Some(tick) = timer_stream.next().await {
                        // Periodic processing
                        let count = results_vec.lock_ref().len();
                        zoon::println!("⏰ Timer tick - {} items processed", count);
                    }
                }
            );
        });
        
        Self { data_events, config_events, timer_events, results }
    }
}
```

### Complete Architecture Example

**Modern File Manager with All Patterns:**
```rust
/// Complete example showing all modern patterns
#[derive(Clone)]
struct ModernFileManager {
    // Core state managed by Actor
    files: ActorVec<TrackedFile>,
    
    // Events using create_with_stream pattern
    pub add_file: Relay<PathBuf>,
    pub remove_file: Relay<String>,
    pub file_selected: Relay<String>,
    
    // Local UI state using SimpleState
    pub filter_text: SimpleState<String>,
    pub is_loading: SimpleState<bool>,
    pub selected_count: SimpleState<usize>,
}

impl Default for ModernFileManager {
    fn default() -> Self {
        // Create streams for all events
        let (add_file, add_stream) = relay();
        let (remove_file, remove_stream) = relay();
        let (file_selected, selection_stream) = relay();
        
        // Create main actor with imperative stream processing
        let files = ActorVec::new(vec![], async move |files_vec| {
            // Process multiple streams concurrently
            future::join!(
                // File addition stream
                async {
                    while let Some(path) = add_stream.next().await {
                        let file = TrackedFile::new(path);
                        files_vec.lock_mut().push_cloned(file);
                    }
                },
                
                // File removal stream
                async {
                    while let Some(file_id) = remove_stream.next().await {
                        files_vec.lock_mut().retain(|f| f.id != file_id);
                    }
                },
                
                // Selection tracking stream
                async {
                    while let Some(file_id) = selection_stream.next().await {
                        zoon::println!("📁 File selected: {}", file_id);
                        // Could update selection state here
                    }
                }
            );
        });
        
        Self {
            files,
            add_file,
            remove_file,
            file_selected,
            // SimpleState for all local UI state
            filter_text: SimpleState::new(String::new()),
            is_loading: SimpleState::new(false),
            selected_count: SimpleState::new(0),
        }
    }
}

impl ModernFileManager {
    // Clean API using direct relay access
    pub fn add_file(&self, path: PathBuf) {
        self.add_file.send(path);
    }
    
    pub fn remove_file(&self, id: String) {
        self.remove_file.send(id);
    }
    
    // Reactive state access
    pub fn files_signal_vec(&self) -> impl SignalVec<Item = TrackedFile> {
        self.files.signal_vec()
    }
    
    pub fn filtered_files_signal(&self) -> impl Signal<Item = Vec<TrackedFile>> {
        map_ref! {
            let files = self.files.signal_vec().to_signal_cloned(),
            let filter = self.filter_text.signal() => {
                // Implement filtering logic using reactive signals
                files.into_iter()
                    .filter(|f| f.name.contains(&*filter))
                    .collect()
            }
        }
    }
}
```

### Performance Benefits

**Measured Improvements from Pattern Adoption:**

1. **Reduced Boilerplate**: ~60% less code using `relay()` vs clone! macros
2. **Better Debugging**: Imperative while loops easier to step through than nested async closures  
3. **Cleaner Error Handling**: Direct relay access eliminates Result<(), RelayError> propagation
4. **Unified State Management**: SimpleState eliminates inconsistent Mutable usage patterns
5. **Concurrent Processing**: join!() pattern enables true multi-stream concurrency

### Migration Strategy

**Step-by-Step Modernization:**

1. **Replace clone! patterns** with `relay()`
2. **Convert .for_each() to while loops** for easier debugging
3. **Introduce SimpleState** for all local UI state (dialog open/closed, filter text, etc.)
4. **Use join!() for multi-stream** scenarios instead of multiple Task::start calls
5. **Eliminate raw Mutable usage** in favor of either Actor (shared state) or SimpleState (local state)

This approach provides the cleanest path forward for new Actor+Relay implementations and systematic modernization of existing code.

#### Advanced Multi-Stream Pattern with select!()

**For Complex Apps: Multiple Related Streams with select!()**
```rust
use futures::select;

/// Advanced pattern for handling multiple related event streams
/// Use when streams need to be processed with different priorities or shared state
#[derive(Clone)]
struct AdvancedCounter {
    pub value: Actor<i32>,
    
    // Multiple related events that need coordinated handling
    pub increment: Relay,
    pub decrement: Relay,
    pub reset: Relay,
    pub multiply: Relay<i32>,
}

impl Default for AdvancedCounter {
    fn default() -> Self {
        let (increment, mut increment_stream) = relay();
        let (decrement, mut decrement_stream) = relay();
        let (reset, mut reset_stream) = relay();
        let (multiply, mut multiply_stream) = relay();
        
        // select! for coordinated multi-stream processing
        let value = Actor::new(0, async move |state| {
            loop {
                select! {
                    Some(()) = increment_stream.next() => {
                        state.update(|value| value + 1);
                    }
                    Some(()) = decrement_stream.next() => {
                        state.update(|value| value - 1);
                    }
                    Some(()) = reset_stream.next() => {
                        state.set_neq(0);  // Reset takes priority
                    }
                    Some(factor) = multiply_stream.next() => {
                        state.update(|value| value * factor);
                    }
                }
            }
        });
        
        AdvancedCounter { value, increment, decrement, reset, multiply }
    }
}
```

### Multi-Stream Pattern Decision Guide

```
Need multiple streams in Actor?
├── Do streams share state or need coordination?
│   ├── YES → Use select!() for coordinated processing
│   │   ├── Different priorities? → Order select! arms by priority
│   │   ├── Complex coordination? → Consider separate coordinatIng Actor
│   │   └── Simple shared state? → Cache values between stream events
│   └── NO → Can streams run independently?
│       ├── YES → Use join!() for concurrent processing  
│       ├── Same event type, multiple sources? → Use futures::stream::select()
│       └── Sequential only? → Use while let with single stream
└── Single stream only? → Use while let Some(event) = stream.next().await
```

**When to use each pattern:**
- **select!()**: Shared state access, coordinated handling, event priorities
- **join!()**: Independent processing, concurrent streams, no shared state  
- **futures::stream::select()**: Same event type from multiple sources
- **while let**: Single stream, simple sequential processing

#### Discrete Event Pattern (Unit Relays)

**For Button-Style Events: Relay<()> Pattern**
```rust
/// Pattern for discrete user actions where only the action matters, not data
struct UserInterface {
    // State
    pub mode: Actor<AppMode>,
    
    // Discrete action events - just notifications
    pub save_clicked: Relay,      // Relay<()> is default
    pub load_clicked: Relay,
    pub exit_clicked: Relay,
    pub help_clicked: Relay,
}

impl Default for UserInterface {
    fn default() -> Self {
        let (save_clicked, mut save_stream) = relay();
        let (load_clicked, mut load_stream) = relay();
        let (exit_clicked, mut exit_stream) = relay();
        let (help_clicked, mut help_stream) = relay();
        
        let mode = Actor::new(AppMode::Normal, async move |state| {
            loop {
                select! {
                    Some(()) = save_stream.next() => {
                        state.set_neq(AppMode::Saving);
                        // Perform save operation
                        perform_save().await;
                        state.set_neq(AppMode::Normal);
                    }
                    Some(()) = load_stream.next() => {
                        state.set_neq(AppMode::Loading);
                        // Perform load operation
                        perform_load().await;
                        state.set_neq(AppMode::Normal);
                    }
                    Some(()) = exit_stream.next() => {
                        state.set_neq(AppMode::Exiting);
                        // Cleanup and exit
                        cleanup().await;
                    }
                    Some(()) = help_stream.next() => {
                        state.set_neq(AppMode::ShowingHelp);
                        // Show help dialog
                        Timer::sleep(3000).await;  // Auto-hide after 3s
                        state.set_neq(AppMode::Normal);
                    }
                }
            }
        });
        
        UserInterface { mode, save_clicked, load_clicked, exit_clicked, help_clicked }
    }
}

// Usage: Just send unit values
ui.save_clicked.send(());
ui.help_clicked.send(());
```

**Benefits of Unit Relays:**
- **Clear intent**: Action-based, not data-based
- **Simple UI binding**: `.on_press(|| relay.send(()))`
- **No ceremony**: No custom event types needed
- **Atomic operations**: Single responsibility per relay

This approach is perfect for button clicks, menu selections, dialog actions, and other discrete user interface events.

## Type Safety

### Strong Type IDs

Replace string IDs with newtype wrappers for compile-time safety:

```rust
// Instead of String IDs everywhere
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScopeId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VariableId(String);

impl FileId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Type-safe messages
#[derive(Clone)]
struct RemoveFile {
    id: FileId,  // Not String!
}

#[derive(Clone)]
struct SelectVariable {
    file: FileId,
    scope: ScopeId,
    variable: VariableId,
}
```

### Error Handling

#### Relay Error Handling Patterns

**Use `.send()` for UI Events (Recommended Default):**
```rust
// UI event handlers - errors auto-logged to console
button().on_press({
    let relay = add_file_relay.clone();
    move || relay.send(file_path.clone())  // Clean - no Result handling needed
})

// Input changes
input().on_change({
    let relay = text_changed_relay.clone();
    move |text| relay.send(text)  // Simple and clean
})
```

**Use `.try_send()` for Critical Operations:**
```rust
// Critical operations that need explicit error handling
pub fn save_critical_data(&self, data: ImportantData) -> Result<(), DataError> {
    self.save_relay.try_send(data)
        .map_err(|e| DataError::RelayFailed(e))?;
    Ok(())
}

// Network operations
pub async fn sync_with_server(&self) {
    if let Err(e) = self.sync_relay.try_send(SyncCommand::Start) {
        show_error_dialog(&format!("Failed to start sync: {}", e));
        return;
    }
    // Continue with sync...
}
```

**Error Types for Complex Operations:**
```rust
// Config serialization
pub async fn save_config(config: &WorkspaceConfig) -> Result<(), ConfigError> {
    let serialized = serde_json::to_string(config)
        .map_err(ConfigError::Serialization)?;
    
    send_up_msg(UpMsg::SaveConfig(serialized))
        .await
        .map_err(ConfigError::Backend)?;
    
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to serialize config: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Backend communication failed: {0}")]
    Backend(String),
    
    #[error("Config file not found")]
    NotFound,
    
    #[error("Relay communication failed: {0}")]
    RelayFailed(#[from] RelayError),
}
```

#### When to Use Each Pattern

| Pattern | Use Case | Example |
|---------|----------|---------|
| `.send()` | UI interactions, non-critical events | Button clicks, input changes, menu selections |
| `.try_send()` | Critical operations, explicit error handling needed | File saves, network requests, validation failures |
| Result types | Complex operations with multiple failure modes | Config loading, data processing, external API calls |

#### Implementation Details

The Relay implementation automatically logs errors to help with debugging:

```rust
impl<T> Relay<T> {
    pub fn send(&self, value: T) {
        if let Err(e) = self.try_send(value) {
            zoon::println!("⚠️ Relay send failed: {:?}", e);
            // Could also add stack trace in debug builds
        }
    }
    
    pub fn try_send(&self, value: T) -> Result<(), RelayError> {
        // Actual implementation with proper error handling
    }
}
```

This provides the best of both worlds: clean UI code with automatic error visibility, and explicit control when needed.


## Migration Patterns

### Pattern 1: Global Message Queue → Structural Relays

**Before:**
```rust
// Stringly-typed message queue
pub enum FileUpdateMessage {
    Add { tracked_file: TrackedFile },
    Update { file_id: String, new_state: FileState },
    Remove { file_id: String },
}

fn send_file_update_message(message: FileUpdateMessage) {
    FILE_UPDATE_QUEUE.lock_mut().push(message);
}
```

**After:**
```rust
// Clean structural approach - no custom types needed!
struct TrackedFile {
    id: String,
    path: PathBuf,
    state: Actor<FileState>,
    
    // Events as simple notifications
    remove_clicked: Relay,      // Relay<()> by default
    state_changed: Relay,       // Just signals "something changed"
}

impl TrackedFile {
    pub fn new(id: String, path: PathBuf) -> Self {
        // Create relays FIRST
        let remove_clicked = Relay::new();
        let state_changed = Relay::new();
        
        // Create Actor that uses the relays - with async closure syntax
        let state = Actor::new(FileState::Loading, async |_state| {
            // Business logic would be wired here
            // Example: Handle external state changes, file loading, etc.
        });
        
        TrackedFile { id, path, state, remove_clicked, state_changed }
    }
}
```

### Pattern 2: Global Mutables → Domain Structs

**Before:**
```rust
// Global state with uncontrolled access
pub static SELECTED_VARIABLES: Lazy<MutableVec<SelectedVariable>> = lazy::default();

// Multiple mutation points
SELECTED_VARIABLES.lock_mut().push_cloned(var);     // state.rs
SELECTED_VARIABLES.lock_mut().retain(|v| ...);      // views.rs
SELECTED_VARIABLES.lock_mut().clear();              // config.rs
```

**After:**
```rust
// Domain-driven structure with embedded state and events
struct VariableSelection {
    variables: ActorVec<SelectedVariable>,
    
    // Simple event notifications
    add_clicked: Relay,
    clear_clicked: Relay,
    remove_clicked: Relay<String>,  // Pass just the ID
}

impl VariableSelection {
    pub fn new() -> Self {
        // Create relays FIRST
        let add_clicked = Relay::new();
        let clear_clicked = Relay::new();
        let remove_clicked = Relay::new();  // Relay<String>
        
        // Create ActorVec that uses the relays - with async closure syntax
        let variables = ActorVec::new(vec![], async |vars| {
            // Business logic handled here during creation
            // Example: Wire up relay handlers to modify the collection
        });
        
        VariableSelection { variables, add_clicked, clear_clicked, remove_clicked }
    }
}
```

### Pattern 3: Direct UI Mutations → Event Emission

**Before:**
```rust
button().on_press(move || {
    // Direct global mutation
    VARIABLES_SEARCH_FILTER.set_neq(text);
    SELECTED_SCOPE_ID.set_neq(Some(scope_id));
    trigger_some_update();
})
```

**After:**
```rust
button().on_press({
    let relay = search_relay.clone();
    move || relay.send(SearchUpdate(text.clone()))
})
```

### Pattern 4: Config with String Keys → Type-Safe Serde

**Before:**
```rust
// Stringly-typed, error-prone
config.insert("theme", theme.to_string());
config.insert("dock_mode", mode.to_string());
let theme = config.get("theme").parse().unwrap();
```

**After:**
```rust
#[derive(Serialize, Deserialize)]
struct WorkspaceConfig {
    pub theme: Theme,
    pub dock_mode: DockMode,
    pub panel_layouts: PanelLayouts,
}

// Type-safe updates
config.lock_mut().theme = Theme::Dark;
config.lock_mut().dock_mode = DockMode::Bottom;
```

## Implementation Examples

### Example 1: Todo App with Structural Pattern

```rust
use uuid::Uuid;

// Clean domain struct - no custom message types needed!
#[derive(Clone, Debug)]
struct Todo {
    id: Uuid,
    text: Actor<String>,
    completed: Actor<bool>,
    
    // Simple event notifications
    clicked: Relay,           // Default Relay<()>
    text_changed: Relay<String>,
    delete_clicked: Relay,
}

impl Todo {
    pub fn new(id: Uuid, initial_text: String) -> Self {
        // Create relays with streams using modern pattern
        let (clicked, mut clicked_stream) = relay();
        let (text_changed, mut text_stream) = relay();
        let delete_clicked = Relay::new();
        
        let completed = Actor::new(false, async move |state| {
            // Handle toggle clicks - business logic belongs in the Actor
            while let Some(()) = clicked_stream.next().await {
                state.update(|current| !current);  // Atomic operation, no .get() needed
            }
        });
        
        let text = Actor::new(initial_text, async move |state| {
            // Handle text changes - business logic in the Actor
            while let Some(new_text) = text_stream.next().await {
                state.set_neq(new_text);  // Only update if text actually changed
            }
        });
        
        Todo { id, text, completed, clicked, text_changed, delete_clicked }
    }
}

// UI components just connect to the pre-wired relays
fn todo_item(todo: &Todo) -> impl Element {
    Row::new()
        .item(
            Checkbox::new()
                .checked_signal(todo.completed.signal())
                .on_click(|| todo.clicked.send(())) // Just notify!
        )
        .item(
            Input::new()
                .text_signal(todo.text.signal())
                .on_change(|text| todo.text_changed.send(text))
        )
        .item(
            Button::new()
                .label("Delete")
                .on_press(|| todo.delete_clicked.send(()))
        )
}

// Higher level component
struct TodoList {
    todos: ActorVec<Todo>,
    add_clicked: Relay,
}

impl TodoList {
    pub fn new() -> Self {
        // Create relays with streams using modern pattern
        let (add_clicked, mut add_stream) = relay();
        
        let todos = ActorVec::new(vec![], async move |todos_vec| {
            // Handle todo management here - business logic in the ActorVec
            while let Some(()) = add_stream.next().await {
                let new_todo = Todo::new(Uuid::new_v4(), "New todo".to_string());
                todos_vec.lock_mut().push_cloned(new_todo);
            }
        });
        
        TodoList { todos, add_clicked }
    }
}
```

### Example 2: File Manager with Clean Architecture

```rust
#[derive(Clone, Debug)]
enum FileState {
    Loading,
    Ready,
    Error(String),
}

// Clean file structure - no complex message types needed!
#[derive(Clone, Debug)]
struct TrackedFile {
    id: String,
    path: PathBuf,
    state: Actor<FileState>,
    
    // Simple event notifications
    remove_clicked: Relay,        // Just notify, no data needed
    reload_clicked: Relay,        // Trigger reload
    state_changed: Relay,         // State was updated externally
}

impl TrackedFile {
    pub fn new(id: String, path: PathBuf) -> Self {
        // Create relays with streams using modern pattern
        let remove_clicked = Relay::new();
        let (reload_clicked, mut reload_stream) = relay();
        let state_changed = Relay::new();
        
        let path_clone = path.clone();
        let state = Actor::new(FileState::Loading, async move |state_actor| {
            let path = path_clone.clone();
            
            // Business logic: initial load
            // let result = load_file(&path).await;
            // state_actor.set(match result { Ok(_) => FileState::Ready, Err(e) => FileState::Error(e) });
            
            // Handle reload events - business logic belongs HERE in the Actor
            while let Some(()) = reload_stream.next().await {
                state_actor.set_neq(FileState::Loading);
                // let result = load_file(&path).await;
                // state_actor.set_neq(match result { Ok(_) => FileState::Ready, Err(e) => FileState::Error(e) });
            }
        });
        
        TrackedFile { id, path, state, remove_clicked, reload_clicked, state_changed }
    }
}

// File collection manager
#[derive(Clone, Debug)]
struct FileManager {
    files: ActorVec<TrackedFile>,
    
    // Collection-level events
    add_file_clicked: Relay,
    clear_all_clicked: Relay,
}

impl FileManager {
    pub fn new() -> Self {
        // Create relays with streams using modern pattern
        let add_file_clicked = Relay::new();
        let (clear_all_clicked, mut clear_stream) = relay();
        
        let files = ActorVec::new(vec![], async move |files_vec| {
            // Collection business logic belongs HERE in the ActorVec
            while let Some(()) = clear_stream.next().await {
                // Clear all files logic
                files_vec.lock_mut().clear();
            }
        });
        
        FileManager { files, add_file_clicked, clear_all_clicked }
    }
}

// Clean UI - just connect to pre-wired events
fn file_item(file: &TrackedFile) -> impl Element {
    Row::new()
        .item(
            El::new().child_signal(
                file.state.signal().map(|state| match state {
                    FileState::Loading => "⏳ Loading...",
                    FileState::Ready => "✅ Ready", 
                    FileState::Error(_) => "❌ Error",
                })
            )
        )
        .item(El::new().child(file.path.display().to_string()))
        .item(
            Button::new()
                .label("Reload")
                .on_press(|| file.reload_clicked.send(())) // Just notify!
        )
        .item(
            Button::new()
                .label("Remove")
                .on_press(|| file.remove_clicked.send(())) // Simple!
        )
}
```

### Example 3: Multi-Stream Actor with SimpleState Helper

```rust
use std::collections::HashMap;

// Configuration that changes over time
#[derive(Clone, Debug)]
pub struct ProcessingConfig {
    pub enabled: bool,
    pub batch_size: usize,
    pub timeout_ms: u64,
}

// Events that arrive from various sources
#[derive(Clone, Debug)]
pub enum DataEvent {
    ItemReceived { id: String, data: Vec<u8> },
    BatchComplete { batch_id: String },
    ErrorOccurred { error: String },
}

// Processed results
#[derive(Clone, Debug)]
pub struct ProcessedItem {
    pub id: String,
    pub result: String,
    pub timestamp: u64,
}

/// Helper for local UI state that doesn't need Actor overhead
/// Replaces all Mutable usage with this simpler pattern
#[derive(Clone, Debug)]
struct SimpleState<T: Clone> {
    state: Mutable<T>,
}

impl<T: Clone> SimpleState<T> {
    fn new(initial: T) -> Self {
        Self { state: Mutable::new(initial) }
    }
    
    // No .get() method - stay consistent with race condition prevention
    fn set(&self, value: T) {
        self.state.set_neq(value);
    }
    
    fn signal(&self) -> impl Signal<Item = T> {
        self.state.signal()
    }
}

/// Advanced example showing an Actor that processes both:
/// - Reactive state changes from other Actors (config)  
/// - Event streams from Relays (data events)
#[derive(Clone, Debug)]
struct DataProcessor {
    // State managed by this Actor
    processed_items: ActorVec<ProcessedItem>,
    
    // External state sources (other Actors)
    config: Actor<ProcessingConfig>,
    
    // Individual Relays using create_with_stream() pattern
    data_events: Relay<DataEvent>,
    status_updates: Relay<String>,
    results_ready: Relay<Vec<ProcessedItem>>,
    error_occurred: Relay<String>,
    
    // Local UI state using SimpleState helper
    is_processing: SimpleState<bool>,
    last_batch_id: SimpleState<Option<String>>,
}

impl DataProcessor {
    pub fn new(initial_config: ProcessingConfig) -> Self {
        // Create relays with streams - modern pattern avoids clone!
        let (data_events, data_stream) = relay();
        let (status_updates, status_stream) = relay();
        let (results_ready, results_stream) = relay();
        let (error_occurred, error_stream) = relay();
        
        // Create config Actor with its own stream handling
        let config = Actor::new(initial_config, async move |config_state| {
            // Config Actor responds to errors by disabling processing temporarily
            error_stream.for_each(async move |error| {
                zoon::println!("⚠️ Processing error: {}", error);
                let current = config_state.get();
                if current.enabled {
                    config_state.update(|mut cfg| {
                        cfg.enabled = false;
                        cfg
                    });
                    
                    // Auto re-enable after timeout
                    Timer::sleep(current.timeout_ms).await;
                    config_state.update(|mut cfg| {
                        cfg.enabled = true;
                        cfg
                    });
                }
            }).await;
        });
        
        // Create main processing ActorVec using imperative while loop pattern
        let processed_items = ActorVec::new(vec![], {
            let config = config.clone();
            let results_ready = results_ready.clone();
            async move |items_vec| {
                // Modern imperative pattern - easier to debug and maintain
                while let Some(event) = data_stream.next().await {
                    let current_config = config.get();
                    
                    if !current_config.enabled {
                        continue; // Skip processing when disabled
                    }
                    
                    match event {
                        DataEvent::ItemReceived { id, data } => {
                            // Process the item according to current config
                            let result = match process_data(&data, &current_config) {
                                Ok(processed) => processed,
                                Err(err) => {
                                    // Send error without clone! - direct access
                                    error_occurred.send(format!("Failed to process {}: {}", id, err));
                                    continue;
                                }
                            };
                            
                            let processed_item = ProcessedItem {
                                id,
                                result,
                                timestamp: current_timestamp(),
                            };
                            
                            items_vec.lock_mut().push_cloned(processed_item);
                            
                            // Check if we have a full batch
                            let items_len = items_vec.lock_ref().len();
                            if items_len >= current_config.batch_size {
                                let batch: Vec<ProcessedItem> = {
                                    let items = items_vec.lock_ref();
                                    items.iter().cloned().collect()
                                };
                                
                                results_ready.send(batch);
                                items_vec.lock_mut().clear();
                            }
                        },
                        
                        DataEvent::BatchComplete { batch_id } => {
                            // Force emit current batch even if not full
                            let batch: Vec<ProcessedItem> = {
                                let items = items_vec.lock_ref();
                                items.iter().cloned().collect()
                            };
                            
                            if !batch.is_empty() {
                                results_ready.send(batch);
                                items_vec.lock_mut().clear();
                            }
                        },
                        
                        DataEvent::ErrorOccurred { error } => {
                            error_occurred.send(error);
                            // Config Actor will handle disabling processing
                        }
                    }
                }
            }
        });
        
        // Create second Actor for status logging using same pattern
        Task::start(async move {
            while let Some(status) = status_stream.next().await {
                zoon::println!("📊 DataProcessor status: {}", status);
            }
        });
        
        DataProcessor {
            processed_items,
            config,
            data_events,
            status_updates,
            results_ready,
            error_occurred,
            // SimpleState for local UI state - eliminates Mutable usage
            is_processing: SimpleState::new(false),
            last_batch_id: SimpleState::new(None),
        }
    }
    
    // Public API for sending events (using RelayBundle)
    pub fn receive_data(&self, id: String, data: Vec<u8>) {
        self.data_events.send(DataEvent::ItemReceived { id, data });
    }
    
    pub fn complete_batch(&self, batch_id: String) {
        self.data_events.send(DataEvent::BatchComplete { batch_id });
    }
    
    pub fn update_status(&self, message: String) {
        self.status_updates.send(message);
    }
    
    // Subscribe to outputs (using RelayBundle)
    pub fn subscribe_to_results(&self) -> impl Stream<Item = Vec<ProcessedItem>> {
        self.relays.results_ready.subscribe()
    }
    
    pub fn subscribe_to_errors(&self) -> impl Stream<Item = String> {
        self.relays.error_occurred.subscribe()
    }
    
    // Access reactive state
    pub fn config_signal(&self) -> impl Signal<Item = ProcessingConfig> {
        self.config.signal()
    }
    
    pub fn items_signal_vec(&self) -> impl SignalVec<Item = ProcessedItem> {
        self.processed_items.signal_vec()
    }
}

// Helper functions
fn process_data(data: &[u8], config: &ProcessingConfig) -> Result<String, String> {
    if data.is_empty() {
        return Err("Empty data".to_string());
    }
    
    // Simulate processing based on config
    let processed = format!("Processed {} bytes with batch_size {}", 
                          data.len(), config.batch_size);
    Ok(processed)
}

fn current_timestamp() -> u64 {
    // In real code, use proper timestamp
    42
}

// Usage example
async fn example_usage() {
    let initial_config = ProcessingConfig {
        enabled: true,
        batch_size: 3,
        timeout_ms: 5000,
    };
    
    let processor = DataProcessor::new(initial_config);
    
    // Subscribe to results (with proper task handle management)
    let _results_task = Task::start_droppable({
        let results_stream = processor.subscribe_to_results();
        async move {
            results_stream.for_each(async |batch| {
                println!("📦 Received batch of {} items", batch.len());
                for item in batch {
                    println!("  - {}: {}", item.id, item.result);
                }
            }).await;
        }
    });
    
    // Subscribe to errors (with proper task handle management)
    let _error_task = Task::start_droppable({
        let error_stream = processor.subscribe_to_errors();
        async move {
            error_stream.for_each(async |error| {
                println!("❌ Processing error: {}", error);
            }).await;
        }
    });
    
    // Send some data
    processor.receive_data("item1".to_string(), vec![1, 2, 3]);
    processor.receive_data("item2".to_string(), vec![4, 5, 6]);
    processor.receive_data("item3".to_string(), vec![7, 8, 9]);
    
    // This will trigger batch emission (batch_size = 3)
    
    processor.update_status("Processing batch 1".to_string());
    
    // Send more data
    processor.receive_data("item4".to_string(), vec![10, 11]);
    processor.complete_batch("batch2".to_string()); // Force emit incomplete batch
}
```

### Key Patterns Demonstrated

**🔄 Mixed Source Processing:**
- **Actor signals**: Config changes from `config.get()` and `config.signal()`  
- **Relay streams**: Data events using `create_with_stream()` pattern
- **Combined logic**: Processing uses current config state + incoming events

**📡 Modern Multi-Stream Pattern:**
- `relay()` eliminates clone! macro boilerplate
- Imperative `while let Some(event) = stream.next().await` loops instead of functional `.for_each()`
- Direct stream access without TaskHandle management complexity
- Cleaner error handling with direct relay access

**⚡ Reactive Configuration:**
- Config Actor responds to error events by temporarily disabling processing
- Main processor checks config state reactively during event processing  
- Automatic re-enabling after timeout shows temporal reactive patterns

**🔧 SimpleState Helper Pattern:**
- `SimpleState<T>` replaces all Mutable usage for local UI state
- Provides clean API without Actor overhead for simple values
- Better than raw Mutable for type safety and consistency

**🏗️ Imperative vs Functional Styles:**
- **Before**: Complex `.for_each(async move |event| ...)` with clone! macros
- **After**: Simple `while let Some(event) = stream.next().await` imperative loops
- Easier debugging, cleaner error handling, less boilerplate

**Note on Signal/Stream Conversions:**
While this example keeps Signals and Streams separate (recommended), the `signal.to_stream()` method from futures-signals is available when you need to unify all inputs as Streams to use Stream-only combinators like `merge`, `zip`, or `buffer`. This can simplify complex event processing pipelines where everything needs the same type, but be aware that Signal→Stream conversion is lossless while Stream→Signal conversion can be lossy (drops intermediate events).

This pattern is perfect for complex systems where Actors need to process both:
1. **Reactive state changes** from other parts of the system (other Actors)
2. **Discrete events** from UI interactions or external sources (Relays)

### Key Benefits of This Pattern

**🎯 No Custom Message Types**: Events are just notifications, data lives in Actors
**🔧 Compile-time Safety**: Relay identity comes from struct field names  
**⚡ Pre-wired Logic**: Business logic connected during creation, UI just hooks up
**📝 Clear Separation**: Creation (business logic) vs UI (event connection)
**🚀 Simple**: `Relay` defaults to `Relay<()>` for most use cases

This approach eliminates the explosion of typed message structs while maintaining full type safety and clear data flow.

## Testing & Debugging

### Testing Utilities

```rust
/// Mock relay for testing
pub struct MockRelay<T> {
    events: Arc<Mutex<Vec<T>>>,
    auto_respond: Option<Box<dyn Fn(&T) -> Option<T>>>,
}

impl<T: Clone> MockRelay<T> {
    pub fn new() -> Self;
    
    /// Set auto-response function for testing
    pub fn with_auto_response<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> Option<T> + 'static;
    
    /// Get all emitted events
    pub fn events(&self) -> Vec<T>;
    
    /// Clear recorded events
    pub fn clear_events(&self);
    
    /// Simulate emission
    pub fn simulate_emit(&self, event: T);
}

/// Test harness for actors
pub struct ActorTestHarness {
    // Track all actors created during test
    actors: Vec<Box<dyn Any>>,
}

impl ActorTestHarness {
    pub fn new() -> Self;
    
    /// Run test with automatic cleanup
    pub async fn run_test<F, Fut>(&mut self, test: F)
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = ()>;
    
    /// Assert no pending messages
    pub fn assert_quiet(&self);
    
    /// Wait for actor processing
    pub async fn wait_for_processing(&self, timeout_ms: u64);
}
```

### Debug Tracing

```rust
#[cfg(debug_assertions)]
pub struct ActorDebugger {
    /// Enable/disable debug output
    pub fn set_enabled(enabled: bool);
    
    /// Set filter for specific actors
    pub fn set_filter(pattern: &str);
    
    /// Get mutation history for all actors
    pub fn get_global_history() -> Vec<(String, Vec<MutationEvent>)>;
    
    /// Clear all debug data
    pub fn clear_debug_data();
}

// Debug macros
#[cfg(debug_assertions)]
macro_rules! actor_trace {
    ($actor:expr, $msg:expr) => {
        if ActorDebugger::is_enabled() {
            zoon::println!("[{}] {}", $actor.name(), $msg);
        }
    };
}
```

### Testing Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use zoon::*;
    
    #[async_test]
    async fn test_actor_processes_events_sequentially() {
        let (relay, mut stream) = relay();
        let results = Arc::new(Mutex::new(Vec::new()));
        
        let actor = Actor::new(0, {
            let results = results.clone();
            async move |state| {
                while let Some(value) = stream.next().await {
                    // Simulate processing
                    Timer::sleep(10).await;
                    state.update(|current| current + value);
                    
                    // Record result using signal-based access
                    let current_value = state.signal().to_stream().next().await.unwrap();
                    results.lock().unwrap().push(current_value);
                }
            }
        });
        
        // Emit events rapidly
        for i in 1..=5 {
            relay.send(i);
        }
        
        // Wait for all processing using signal-based approach
        let mut value_stream = actor.signal().to_stream();
        let mut final_value = 0;
        for _ in 1..=5 {
            final_value = value_stream.next().await.unwrap();
        }
        
        // Verify final result after all processing
        assert_eq!(final_value, 15); // 1+2+3+4+5 = 15
        let results = results.lock().unwrap().clone();
        assert_eq!(results, vec![1, 3, 6, 10, 15]);
    }
    
    #[async_test]
    async fn test_relay_drops_events_without_subscribers() {
        let relay = Relay::<String>::new();
        
        // No subscribers
        assert!(!relay.has_subscribers());
        
        // With no subscribers, send() will auto-log the error
        relay.send("test".to_string());  // Error logged to console automatically
        
        // For explicit error checking, use try_send()
        let result = relay.try_send("test".to_string());
        assert!(matches!(result, Err(RelayError::NoSubscribers)));
    }
    
    #[async_test]
    async fn test_actor_lifecycle() {
        let (relay, mut stream) = relay();
        let actor = Actor::new(42, async move |_state| {
            // Actor task - wait for events
            while let Some(_) = stream.next().await {
                // Process events
            }
        });
        
        // Test initial value using signal
        let mut value_stream = actor.signal().to_stream();
        assert_eq!(value_stream.next().await.unwrap(), 42);
        
        // Test actor can receive events
        relay.send(());
        Timer::sleep(10).await;
        
        // In real implementation, would have stop() method and is_running() check
        // This shows signal-based testing approach instead of direct .get() access
    }
}
```

### Connection Tracking

For debugging data flow:

```rust
#[cfg(debug_assertions)]
pub struct ConnectionTracker {
    connections: Arc<Mutex<Vec<Connection>>>,
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub source: String,      // e.g., "button#add-file"
    pub relay: String,        // e.g., "add_file_relay"
    pub actor: String,        // e.g., "tracked_files_actor"
    pub timestamp: f64,
}

impl ConnectionTracker {
    /// Record a UI -> Relay connection
    pub fn track_emission(source: &str, relay: &str);
    
    /// Record a Relay -> Actor connection
    pub fn track_subscription(relay: &str, actor: &str);
    
    /// Get all connections for a relay
    pub fn connections_for_relay(relay: &str) -> Vec<Connection>;
    
    /// Visualize connection graph
    pub fn print_graph();
}

// Usage in debug builds
#[cfg(debug_assertions)]
ConnectionTracker::track_emission("file_list_item#123", "remove_file_relay");
```

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

**For global state migration plans, see [actors_and_relays_globals.md](actors_and_relays_globals.md)**

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

## Benefits Summary

### Immediate Benefits
- **No more recursive locks** - Sequential processing eliminates race conditions
- **Traceable mutations** - Every state change has a clear source
- **Reduced over-rendering** - Controlled update propagation
- **Type safety** - Compile-time checking for all state operations

### Long-term Benefits
- **Testability** - Actors can be tested in isolation
- **Maintainability** - Clear data flow and dependencies
- **Debuggability** - Built-in tracing and logging
- **Scalability** - Easy to add new features without global state pollution

## Refactoring Patterns & Antipatterns

Based on practical experience refactoring examples from MoonZoon patterns to Actor+Relay architecture, here are key insights and common pitfalls:

### Modern Pattern Improvements

#### 1. **relay() vs clone! Macro**
```rust
// ❌ Old complex pattern with clone! macro
let relay = Relay::new();
let actor = Actor::new(initial, clone!((relay) async move |state| {
    relay.subscribe().for_each(clone!((state) async move |event| {
        // Complex clone management, nested async blocks
    })).await;
}));

// ✅ New streamlined pattern
let (relay, stream) = relay();
let actor = Actor::new(initial, async move |state| {
    while let Some(event) = stream.next().await {
        // Direct access, clear control flow
    }
});
```
**Benefits**: 60% less boilerplate, easier debugging, cleaner error handling

#### 2. **SimpleState for Eliminating Mutable Usage**
```rust
// ❌ Inconsistent Mutable usage throughout codebase
static DIALOG_OPEN: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
static LOADING: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// ✅ Unified SimpleState pattern (Actor+Relay internally)
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

#### 3. **Multi-Stream Processing with join!()**
```rust
// ❌ Multiple Task::start calls - harder to coordinate
Task::start(async { stream1.for_each(...).await });
Task::start(async { stream2.for_each(...).await });
Task::start(async { stream3.for_each(...).await });

// ✅ Coordinated multi-stream processing
ActorVec::new(vec![], async move |state| {
    future::join!(
        async { while let Some(event) = stream1.next().await { /* process */ } },
        async { while let Some(event) = stream2.next().await { /* process */ } },
        async { while let Some(event) = stream3.next().await { /* process */ } }
    );
});
```

### Simplification Patterns

#### 0. **Type Unification Pattern (from Counters Example)**

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

// Duplicate implementation
impl Default for ColumnControl {
    fn default() -> Self { /* identical setup code */ }
}
impl Default for RowControl {
    fn default() -> Self { /* identical setup code */ }
}

// Usage requires different types
let columns = ColumnControl::default();
let rows = RowControl::default();
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

// Usage: Same type for both
let columns = GridDimensionControl::default();
let rows = GridDimensionControl::default();

// Unified UI helper - no duplication needed
fn dimension_control_counter(control: &GridDimensionControl) -> impl Element {
    Row::new()
        .item(Button::new().label("-").on_press({
            let relay = control.decrement.clone();
            move || relay.send(())
        }))
        .item_signal(control.count.signal())
        .item(Button::new().label("+").on_press({
            let relay = control.increment.clone();
            move || relay.send(())
        }))
}

// Works for both columns and rows - no code duplication
dimension_control_counter(&COLUMNS);
dimension_control_counter(&ROWS);
```

**Benefits:**
- **50% less code**: Single implementation handles both use cases  
- **Single source of truth**: Changes apply to all instances automatically
- **Unified methods**: Helper functions work for all instances
- **Better maintenance**: Bug fixes and features benefit all instances
- **Type safety**: Compile-time guarantee of identical behavior

**When to Apply:**
- When you find identical struct definitions with different names
- When implementations are copy-pasted between types
- When helper functions are duplicated for similar concepts
- When business logic is identical but context differs

#### 1. **Remove Unnecessary Wrapper Types**
```rust
// ❌ Over-abstraction: Wrapper for simple values
struct ChangeBy(i32);
struct SetColumns(usize);
struct SetRows(usize);
struct IncrementCounter { id: String, amount: i32 }

pub change: Relay<ChangeBy>,
pub set_columns: Relay<SetColumns>,
pub increment: Relay<IncrementCounter>,

// ✅ Direct and clear: Use primitive types or consolidated enums
pub change_by: Relay<i32>,
pub set_columns: Relay<usize>,
pub set_rows: Relay<usize>,

// OR for related operations:
pub enum GridAction {
    SetColumns(usize),
    SetRows(usize),
}
pub grid_action: Relay<GridAction>,
```
**Rule**: Only create wrapper types when they add meaningful type safety or behavior. Use primitives for simple values, enums for related operations.

#### 2. **Use Default Trait for Zero-Config Initialization**
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
**Rule**: Use `Default` trait when initialization needs no parameters - mirrors MoonZoon's `lazy::default()` pattern.

#### 3. **Direct Field Access When Safe**
```rust
// ❌ Unnecessary wrapper methods and Result returns
impl Counter {
    pub fn value_signal(&self) -> impl Signal<Item = i32> {
        self.value.signal()  // Just forwarding
    }
    pub fn send_change(&self, amount: i32) {
        self.change_by.send(amount);  // Just forwarding - unnecessary wrapper
    }
    pub fn increment(&self) {
        self.change_by.send(1);  // Helper method with no added value
    }
}

// ✅ Public fields - Actor prevents external mutation, no boilerplate
struct Counter {
    pub value: Actor<i32>,     // Can call .signal() and .get() directly
    pub change_by: Relay<i32>, // Can call .send() directly
}

// Usage is cleaner:
COUNTER.change_by.send(5);     // Direct, obvious
COUNTER.value.signal();        // No indirection
```
**Rule**: Make Actor and Relay fields public when there's no additional logic - they're inherently safe and reduce boilerplate by 50%+.

#### 4. **Flatten Nested Relay Bundles**
```rust
// ❌ Over-organization: Nested relay bundles add complexity
struct GridControlRelays {
    set_columns: Relay<SetColumns>,
    set_rows: Relay<SetRows>,
    reset_all: Relay,
}

struct CounterRelays {
    increment: Relay<IncrementCounter>,
    decrement: Relay<DecrementCounter>,
    reset: Relay<ResetCounter>,
}

struct GridManager {
    config: Actor<GridConfig>,
    counters: ActorVec<CounterData>,
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
**Rule**: Keep all Actor/Relay fields at the same level. Avoid nested "relay bundle" structs that add indirection without value.

#### 5. **Event Consolidation**
```rust
// ❌ Event proliferation: Too many tiny event types
struct IncrementCounter { id: String, amount: i32 }
struct DecrementCounter { id: String, amount: i32 }
struct ResetCounter { id: String }
struct SetCounterValue { id: String, value: i32 }

// Multiple relays for related operations
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

// Usage is still clear:
relay.send(CounterAction::ChangeBy(id, 5));    // increment
relay.send(CounterAction::ChangeBy(id, -3));   // decrement
```
**Rule**: When events share the same handling context (same Actor), consolidate them into enums. This reduces the number of streams to manage and simplifies Actor setup.

### Local State Organization Patterns

#### Local State with Struct Methods (MOST IDIOMATIC - Recommended Default)

The examples clearly demonstrate that struct methods with `self` are the most Rust-idiomatic approach:

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

**Why This is Most Idiomatic Rust:**
- **Natural method organization**: `impl Block` with related methods
- **self parameter usage**: Feels like normal Rust struct patterns
- **No parameter threading**: Methods have direct access to all state
- **Unified type handling**: Single type for similar operations (no ColumnControl vs RowControl)
- **Clean module boundaries**: State and behavior co-located in same struct

**Comparison with Parameter Passing Chaos:**
```rust
// ❌ Parameter Passing Hell (functional approach)
fn root() -> impl Element {
    let counter = Counter::default();
    let ui_state = UIState::default();
    Column::new()
        .item(counter_controls(&counter, &ui_state))
        .item(status_panel(&counter, &ui_state))
}

fn counter_controls(counter: &Counter, ui_state: &UIState) -> impl Element {
    Row::new()
        .item(counter_button("-", -1, counter))  // Threading parameters
        .item_signal(counter.value.signal())
        .item(counter_button("+", 1, counter))   // Threading parameters
}

// vs

// ✅ Self Methods (idiomatic Rust approach)
impl CounterApp {
    fn counter_controls(&self) -> impl Element {
        Row::new()
            .item(self.counter_button("-", -1))  // Clean self access
            .item_signal(self.value.signal())
            .item(self.counter_button("+", 1))
    }
}
```

**Benefits:**
- **Most idiomatic Rust**: Struct methods with `self` - feels natural to Rust developers
- **Zero parameter passing**: Direct access to all state through `self`
- **Better encapsulation**: State scoped to component lifecycle, not global
- **Easier testing**: Each instance can be tested in isolation
- **Cleaner refactoring**: Moving methods between structs is straightforward
- **No lifetime complexity**: `|| CounterApp::default().root()` works perfectly
- **Type unification**: Single types handle multiple similar use cases
- **Clear ownership**: Obvious lifetime and mutation boundaries

### Critical Antipatterns to Avoid

#### 1. **Using clone! Macro Instead of relay()**
```rust
// ❌ DEPRECATED: Complex clone! pattern - harder to debug
let relay = Relay::new();
Actor::new(initial, clone!((relay) async move |state| {
    relay.subscribe().for_each(clone!((state) async move |event| {
        // Nested async blocks, complex clone management
    })).await;
}));

// ✅ MODERN: Direct stream access - much cleaner
let (relay, stream) = relay();
Actor::new(initial, async move |state| {
    while let Some(event) = stream.next().await {
        // Simple imperative loop, direct access
    }
});
```
**Critical**: Always use `create_with_stream()` for new code - it eliminates clone! macro complexity.

#### 1.5. **Multiple relay.send() Calls from Same Source Location**

**⚠️ CRITICAL CONSTRAINT**: A Relay can only be sent from ONE source location in your code.

```rust
// ❌ WRONG: Will panic with "multiple source" error
fn test() {
    relay.send(()); // First source location  
    relay.send(()); // PANIC: Multiple source locations detected
}

// ✅ CORRECT: Different approaches for multiple sends
// Option 1: Use Task::start for different call sites
for _ in 0..3 {
    Task::start(async { relay.send(()); });
}

// Option 2: Batch relay with count parameter
batch_relay.send(3); // Send count instead of multiple individual sends

// Option 3: futures::stream::select for multiple triggers (like chat example)
let send_trigger_stream = futures::stream::select(
    enter_pressed_stream,
    send_button_clicked_stream
);
```

**Why this rule exists**: Relay enforces single ownership to prevent conflicting event sources and maintain clear event traceability.

#### 2. **Raw Mutable Instead of SimpleState**
```rust
// ❌ INCONSISTENT: Raw Mutable scattered throughout
static DIALOG_STATE: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
static FILTER: Lazy<Mutable<String>> = lazy::default();

// ✅ UNIFIED: SimpleState for consistent local state management
struct UIState {
    dialog_open: SimpleState<bool>,
    filter_text: SimpleState<String>,
}
```
**Critical**: Use SimpleState for all local UI state - eliminates inconsistent Mutable patterns.

#### 3. **Task::start_droppable in Actor Setup**
```rust
// ❌ BROKEN: TaskHandle drops immediately, cancelling subscription
Actor::new(0, |state| async move {
    Task::start_droppable(async move {
        relay.subscribe().for_each(...).await;
    }); // TaskHandle dropped here - subscription cancelled!
});

// ✅ CORRECT: Await subscription directly or use create_with_stream()
let (relay, stream) = relay();
Actor::new(0, async move |state| {
    while let Some(event) = stream.next().await {
        // Process events
    }
});
```
**Critical**: Never wrap subscriptions in `Task::start_droppable` within Actor setup - the task will be immediately cancelled.

#### 4. **Separate Get/Set Operations (Race Conditions)**
```rust
// ❌ RACE CONDITION: Value can change between get() and set()
let current = state.get();
state.set_neq(current + amount);

// ✅ ATOMIC: Single operation, no races possible
state.update(|value| value + amount);
```
**Rule**: Always use `update()` for read-modify-write operations - matches `lock_mut()` patterns from original code.

#### 6. **Helper Methods Instead of Direct Field Access**
```rust
// ❌ DEPRECATED: Wrapper methods that just forward calls
impl GridManager {
    pub fn set_columns(&self, cols: usize) -> Result<(), RelayError> {
        self.grid_controls.set_columns.send(SetColumns(cols))  // Just forwarding
    }
    
    pub fn config_signal(&self) -> impl Signal<Item = GridConfig> {
        self.config.signal()  // Just forwarding
    }
    
    pub fn increment_counter(&self, id: &str) -> Result<(), RelayError> {
        self.counter_controls.increment.send(IncrementCounter {
            id: id.to_string(),
            amount: 1,
        })  // Boilerplate with no added value
    }
}

// ✅ MODERN: Direct field access - cleaner and more obvious
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
**Critical**: Don't create wrapper methods for simple field access. Direct field usage is clearer and reduces code by 60%+.

#### 7. **Over-Abstraction Early**
```rust
// ❌ Premature complexity
enum CounterAction {
    Increment(i32),
    Decrement(i32), 
    Reset,
    SetTo(i32),
}

// ✅ Start simple, add complexity when needed
pub change_by: Relay<i32>,  // Handles +1, -1, +5, etc.
```
**Rule**: Begin with the simplest pattern that works. Add abstractions when you have multiple concrete use cases.

### Atomic Operation Patterns

#### The .get() Race Condition Problem

**⚠️ CRITICAL: Actor<T> intentionally does NOT provide a .get() method to prevent race conditions.**

```rust
// ❌ RACE CONDITION ANTIPATTERN (examples show this but it's dangerous)
// If .get() existed, this would be wrong:
let current = counter.value.get();  // Read current value
counter.change_by.send(current + 1); // Send new value - RACE CONDITION!
// Problem: Value can change between get() and send()

// ❌ WRONG: Testing code that assumes .get() exists
assert_eq!(counter.value.get(), 3); // .get() is NOT in Actor API

// ✅ CORRECT: Atomic operations using state.update() inside Actor
let (change_by, mut change_stream) = relay();
let value = Actor::new(0, async move |state| {
    while let Some(amount) = change_stream.next().await {
        // This is atomic - no race conditions possible
        state.update(|current| current + amount);
    }
});

// ✅ CORRECT: Separate atomic operations for common patterns
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

#### When .get() Might Be Needed

**For Testing Only:**
```rust
// If testing requires direct value access, consider adding to Actor API:
impl<T> Actor<T> {
    #[cfg(test)]  // Only available in tests
    pub fn get(&self) -> T {
        self.state.get()  // Direct access for assertions
    }
}

// Usage in tests:
#[cfg(test)]
mod tests {
    #[async_test]
    async fn test_counter() {
        let counter = Counter::default();
        counter.increment.send(());
        let final_value = counter.value.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, 1);
    }
}
```

**For Simple Local State (SimpleState):**
```rust
// SimpleState also avoids .get() for consistency - even local UI state should be reactive
struct SimpleState<T> {
    state: Mutable<T>,
}

impl<T: Clone> SimpleState<T> {
    fn set(&self, value: T) { self.state.set_neq(value); }
    fn signal(&self) -> impl Signal<Item = T> { self.state.signal() }
    // No .get() method - prefer reactive patterns throughout
}
```

#### Matching Original MoonZoon Patterns
```rust
// Original: Atomic mutation
*COUNTER.lock_mut() += step;

// Actor+Relay: Equivalent atomic update
state.update(|value| value + step);
```

#### Safe Arithmetic Patterns (from Examples)

The examples demonstrate much cleaner arithmetic using standard library methods instead of type conversions:

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

**Benefits:**
- **No type conversions**: Stay in native types (usize, i32) throughout
- **Overflow protection**: `saturating_*` methods handle edge cases
- **Cleaner code**: Standard library methods express intent clearly
- **No casting bugs**: Eliminates `as i32` / `as usize` error potential
- **Better performance**: No unnecessary conversions

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

#### Complex State Updates
```rust
// ✅ Multiple fields updated atomically
state.update(|current_state| ComplexState {
    counter: current_state.counter.saturating_add(amount),
    last_updated: now(),
    history: updated_history,
    ..current_state
});
```

### Naming Conventions

Based on patterns observed in production Actor+Relay implementations, these naming conventions provide consistency and clarity:

#### Component Naming Patterns

**Core Components:**
```rust
// ✅ Standard suffixes for different component types
struct ChatApp {
    // State: *_actor for single values, without suffix for collections
    username_actor: Actor<String>,
    message_text_actor: Actor<String>, 
    messages_actor: ActorVec<Message>,        // Collection actors
    viewport_y_actor: Actor<i32>,
    
    // Events: *_relay for all relay types
    enter_pressed_relay: Relay,               // Unit relays (Relay<()>)
    send_button_clicked_relay: Relay,
    username_input_changed_relay: Relay<Username>,  // Typed relays
    message_input_changed_relay: Relay<MessageText>,
    message_received_relay: Relay,
    message_sent_relay: Relay,
    
    // Streams: *_stream when using create_with_stream() pattern
    // (streams are typically private implementation details)
}

// External integrations: *_adapter for service bridges
connection_adapter: ConnectionAdapter<UpMsg, DownMsg>,
websocket_adapter: WebSocketAdapter,
file_system_adapter: FileSystemAdapter,
```

#### Descriptive Relay Names
```rust
// ❌ Generic names
pub events: Relay<SomeEvent>,
pub actions: Relay<Action>,
pub data: Relay<Data>,

// ✅ Descriptive names indicating purpose and type
pub file_added: Relay<PathBuf>,           // Event-based: what happened
pub text_changed: Relay<String>,          // Change-based: what property changed
pub button_clicked: Relay,                // Action-based: user interaction
pub zoom_level_changed: Relay<f64>,       // Property change with value
pub selection_cleared: Relay,             // State change notification

// ✅ Consistent patterns for common operations
pub item_selected: Relay<ItemId>,         // Selection events
pub item_deselected: Relay<ItemId>,
pub filter_text_changed: Relay<String>,   // Input changes
pub config_updated: Relay<ConfigSection>, // Configuration changes
pub error_occurred: Relay<ErrorMessage>,  // Error events
```

#### Actor Naming Patterns
```rust
// ✅ Clear state ownership with *_actor suffix for single values
username_actor: Actor<String>,
cursor_position_actor: Actor<f64>, 
theme_actor: Actor<Theme>,
config_actor: Actor<WorkspaceConfig>,

// ✅ Collection actors: use descriptive name + actor suffix
files_actor: ActorVec<TrackedFile>,       // OR tracked_files_actor
variables_actor: ActorVec<Variable>,      // OR selected_variables_actor  
scopes_actor: ActorBTreeMap<ScopeId, Scope>,

// ✅ Alternative: omit _actor suffix for collections when clear from context
messages: ActorVec<Message>,              // Clear from type
counters: ActorVec<Counter>,              // Context makes it obvious
```

#### Stream Naming (Internal Implementation)
```rust
// ✅ When using create_with_stream(), name streams consistently
let (add_file_relay, mut add_file_stream) = relay();
let (remove_file_relay, mut remove_file_stream) = relay();
let (selection_changed_relay, mut selection_changed_stream) = relay();

// ✅ External service streams: describe the source
let (connection_adapter, mut incoming_message_stream) = ConnectionAdapter::new();
let mut file_watcher_stream = create_file_watcher_stream();
let mut timer_tick_stream = create_timer_stream(Duration::from_secs(1));
```

#### Struct Naming
```rust
// ❌ Redundant suffixes
struct CounterState { ... }      // "State" is redundant
struct FileManagerService { ... } // "Service" doesn't add clarity
struct UserInterface { ... }     // Too generic

// ✅ Direct, clear names
struct Counter { ... }           // Clear and concise
struct FileManager { ... }       // Describes responsibility
struct ChatApp { ... }           // Application-level component
struct DialogState { ... }       // OK when distinguishing from Dialog component
```

#### Type Alias Patterns
```rust
// ✅ Use type aliases for frequently passed data to reduce clone overhead
type Username = Arc<String>;
type MessageText = Arc<String>;  
type FilePath = Arc<PathBuf>;

// ✅ Strong type IDs for compile-time safety
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FileId(String);
#[derive(Debug, Clone, PartialEq, Eq, Hash)] 
struct ScopeId(String);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VariableId(String);
```

### Testing Patterns

#### Essential Testing Protocol (from Examples)

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

#### Why signal().next().await Works

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

#### Proper Test Structure
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

#### Test Organization
- **Separate tests into dedicated sections** for documentation clarity
- **Use consistent initialization** (`Default::default()`)
- **Always use signal-based testing** with `.signal().to_stream().next().await`
- **Test both positive and negative cases** (increment/decrement)

### Migration Strategy

#### Relay Creation - The Idiomatic Pattern

The `relay()` function follows Rust's established channel creation pattern:

```rust
// Standard Rust channels
let (tx, rx) = channel();

// Actor+Relay pattern - identical ergonomics
let (relay, stream) = relay();
```

This is the only way to create a Relay with its associated stream. The pattern is:
- **Consistent**: Matches Rust's channel conventions
- **Simple**: One clear way to do things
- **Ergonomic**: Short, memorable function name

#### Incremental Approach
1. **Start with isolated components** (like Counter example)
2. **Keep it simple initially** - avoid premature optimization
3. **Test each refactoring step** before adding complexity
4. **Remove abstractions that don't add value**

#### Common Migration Steps
1. **Identify the core state** (what was in the `Mutable`)
2. **Define minimal event types** (start with primitives)
3. **Create Actor with Default impl** (match original patterns)
4. **Use direct field access** (public Actor/Relay fields)
5. **Replace separate get/set with atomic update**
6. **Add tests in separate section**

### Lessons Learned

The most effective Actor+Relay implementations are often simpler than initial attempts. The pattern's power comes from clear ownership and event traceability, not complex abstractions. 

**Key insights from pattern evolution:**

1. **Imperative > Functional**: `while let Some(event) = stream.next().await` is cleaner and more debuggable than nested `.for_each()` closures
2. **Direct Stream Access**: `create_with_stream()` eliminates 60% of boilerplate compared to clone! macro patterns
3. **Unified State Management**: SimpleState provides consistency for local UI state, Actor for shared state
4. **Multi-Stream Coordination**: `join!()` enables true concurrent stream processing vs independent Task::start calls

Start simple, stay atomic, and let the architecture's benefits emerge naturally. Modern patterns make the transition from MoonZoon's global state even cleaner and more maintainable.

## Performance Best Practices

Based on patterns observed in the examples, these practices optimize Actor+Relay performance:

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

// Grid calculation for 2D access
fn grid_index(row: usize, col: usize, columns: usize) -> usize {
    row * columns + col
}
```

**❌ Avoid string IDs for frequent operations:**
```rust
// BAD: String ID lookup - O(n) search performance
struct CounterGrid {
    values: ActorBTreeMap<String, i32>,  // String lookup overhead
    change: Relay<(String, i32)>,        // ID + amount
}
```

### Lifetime Simplification Patterns

**✅ Actor Arc internals enable simple lifetime patterns:**
```rust
// WORKS: Simple pattern enabled by Arc internally
fn main() {
    start_app("app", || CounterApp::default().root());
    //                   ^^^ Creates instance, calls method, returns Element
    //                       Works because Actor<T> is Arc<Mutable<T>> internally
}

// WORKS: Direct instantiation in closures
button().on_press(|| {
    CounterApp::default().some_method();  // Safe because Arc-based
});
```

**❌ Don't over-engineer lifetimes:**
```rust
// UNNECESSARY: Complex lifetime management not needed
struct AppWrapper<'a> {
    counter: &'a Counter,  // Reference not needed
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
    .map(|v| v + 1)
    .map(|v| v * 2)  
    .map(|v| v.to_string())
    .map(|s| format!("Value: {}", s))  // 4 operations per change!
```

### Testing Performance

**✅ Reactive waiting - no arbitrary timeouts:**
```rust
// GOOD: Wait exactly as long as needed
let result = counter.value.signal().to_stream().next().await.unwrap();

// GOOD: Natural batching with signal waiting
counter.increment.send(());
counter.increment.send(());  
counter.decrement.send(());
let result = counter.value.signal().to_stream().next().await.unwrap();  // Waits for final result
```

**✅ Multiple assertions with signal streams:**
```rust
// Test state changes over time
let mut signal_stream = counter.value.signal().to_stream();

counter.increment.send(());
assert_eq!(signal_stream.next().await.unwrap(), 1);

counter.increment.send(()); 
assert_eq!(signal_stream.next().await.unwrap(), 2);
```

These patterns, observed consistently across the examples, provide the foundation for high-performance Actor+Relay applications.

## ✅ Consolidated Best Practices Summary

### **Architecture Fundamentals**
- **All state through Actors**: No raw `Mutable<T>` usage - use `SimpleState` helper for local UI state
- **Signal-only state access**: No `.get()` methods - all state access through `.signal()` and signal-based testing
- **Atomic operations**: Use `state.update()` for modifications, never get-modify-set patterns
- **Local state by default**: Use struct methods with local state, global only when truly needed

### **Modern Implementation Patterns**  
- **relay()**: Always use instead of clone! macro patterns for cleaner Actor initialization
- **Imperative stream processing**: Use `while let Some(event) = stream.next().await` instead of `.for_each()` 
- **SimpleState consistency**: Use Actor+Relay internally for all local UI state (button hover, dialog open/closed)
- **Unified types**: Single type for similar operations (avoid ColumnControl vs RowControl duplication)

### **Multi-Stream Decision Guide**
```
Multiple streams needed?
├── Shared state/coordination? → Use select!()
├── Independent processing? → Use join!()
├── Same event type, multiple sources? → Use futures::stream::select()
└── Single stream? → Use while let
```

### **Error Handling Strategy**
- **UI events**: Use `.send()` (auto-logged errors)
- **Critical operations**: Use `.try_send()` (explicit error handling)
- **Keep it simple**: Don't over-engineer - basic error states and retry patterns

### **Testing Approach**
- **Signal-based testing**: Use `actor.signal().to_stream().next().await` for assertions
- **ActorVec testing**: Use `actor_vec.signal_vec_cloned().to_signal_cloned().to_stream().next().await` for vector assertions
- **Single source location**: Relay can only be sent from ONE location - use Task::start loops or batch relays for multiple sends
- **No timing dependencies**: Wait for actual signal changes, not arbitrary timeouts
- **Reactive waiting**: Natural batching with signal stream testing

### **Type Safety & Performance**
- **Saturating arithmetic**: Always use `.saturating_add()`, `.saturating_sub().max(1)` for safe math
- **Type unification**: Pure `usize` for counts, avoid unnecessary `i32`/`usize` conversions  
- **Vec indices**: Use index-based operations for performance instead of string ID lookups
- **Direct field access**: Make Actor/Relay fields public - they're inherently safe

### **Anti-Patterns to Avoid**
- ❌ Raw `Mutable<T>` usage anywhere in Actor+Relay code
- ❌ clone! macro patterns - use `create_with_stream()` instead  
- ❌ `.get()` method usage - signal-based access only
- ❌ Complex *Manager/*Service patterns - keep business logic simple
- ❌ Multiple Task::start calls - use `join!()` for coordination
- ❌ Timer-based testing - use signal streams for deterministic tests

### **Migration Strategy**
1. **Start with SimpleState**: Replace local Mutable usage first
2. **Apply relay()**: Update Actor initialization patterns  
3. **Remove .get() calls**: Convert to signal-based access
4. **Local state refactor**: Move from global to local state architecture
5. **Testing conversion**: Signal-based testing throughout

This architecture provides **traceability**, **controlled mutations**, **no recursive locks**, and **better testability** while maintaining Zoon's reactive programming model.

## Conclusion

The Actor/Relay architecture solves NovyWave's current state management problems while maintaining Zoon's reactive programming model. By providing clear ownership, type-safe message passing, and controlled mutation points, this architecture will make the codebase more maintainable, debuggable, and reliable.

The migration can be done incrementally, starting with the most problematic areas (file management) and gradually expanding to cover the entire application. Each migrated component becomes more testable and maintainable, providing immediate value even before the full migration is complete.