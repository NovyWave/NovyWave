# Actor+Relay Architecture

This document covers the **core concepts and API design** for Actor+Relay architecture. For specialized topics, see the related documentation below.

## 📚 Related Documentation

- **[Actor+Relay Patterns](actor_relay_patterns.md)** - Migration strategies and modern patterns
- **[Implementation Examples](actor_relay_examples.md)** - Complete working examples
- **[Testing Guide](actor_relay_testing_guide.md)** - Testing and debugging strategies  
- **[External API Bridging](actors_and_relays_bridging.md)** - ConnectionAdapter and external service integration
- **[Global State Patterns](actors_and_relays_globals.md)** - Global state approaches (bridge documentation)
- **[Refactoring Guide](.claude/extra/technical/actor-relay-refactoring-guide.md)** - Step-by-step migration and antipatterns

## Table of Contents
1. [Motivation & Problem Analysis](#motivation--problem-analysis)
2. [Module Structure](#module-structure)
3. [Architecture Overview](#architecture-overview)
4. [API Design](#api-design)
5. [SimpleState Helper](#simplestate-helper)
6. [Type Safety](#type-safety)

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

## SimpleState Helper

### ⚠️ CRITICAL RULE: NO RAW MUTABLES ⚠️

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

#### Complete SimpleState Implementation

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

## Summary

The Actor+Relay architecture provides:
- **Single point of mutation** per state
- **Event traceability** for all state changes  
- **No recursive locks** through sequential processing
- **Type safety** with compile-time message checking
- **Clean testing** through signal-based assertions
- **Architectural consistency** via the SimpleState helper

For implementation details, migration strategies, and working examples, see the related documentation listed at the top of this file.