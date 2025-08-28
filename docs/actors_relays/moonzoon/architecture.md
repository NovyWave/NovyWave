# Actor+Relay Architecture

This document covers the **core concepts and API design** for Actor+Relay architecture - a reactive state management pattern for MoonZoon applications.

## 📚 Related Documentation

- **[Actor+Relay Patterns](patterns.md)** - Migration strategies and modern patterns
- **[Implementation Examples](examples.md)** - Complete working examples
- **[Testing Guide](testing_guide.md)** - Testing and debugging strategies  
- **[External API Bridging](bridging.md)** - ConnectionAdapter and external service integration
- **[Refactoring Guide](refactoring.md)** - Step-by-step migration and antipatterns

## Table of Contents
1. [Motivation & Problem Analysis](#motivation--problem-analysis)
2. [Module Structure](#module-structure)
3. [Architecture Overview](#architecture-overview)
4. [API Design](#api-design)
5. [Atom Helper](#atom-helper)
6. [Type Safety](#type-safety)

## Motivation & Problem Analysis

### Common Problems in MoonZoon Applications

Large reactive applications often suffer from architectural issues stemming from uncontrolled global state mutations:

#### 1. **Unclear Mutation Sources** (Multiple Global Mutables)
```rust
// Common antipattern: Who modifies SHARED_DATA? Multiple places!
pub static SHARED_DATA: Lazy<MutableVec<Item>> = lazy::default();

// Multiple mutation points across files - impossible to trace
SHARED_DATA.lock_mut().push_cloned(item);    // component_a.rs:42
SHARED_DATA.lock_mut().retain(|i| ...);      // service.rs:133  
SHARED_DATA.lock_mut().set_cloned(i, ...);   // config.rs:67
```

#### 2. **Recursive Lock Panics**
```rust
// Problematic pattern causing runtime panics:
SHARED_DATA.signal_vec_cloned().for_each_sync(|items| {
    // This runs while parent lock is still held!
    SHARED_DATA.lock_mut().update();  // PANIC: Recursive lock!
});
```

#### 3. **Over-Rendering Issues**
```rust
// Signal cascade causing excessive UI updates:
SHARED_DATA → DERIVED_STATE → child_signal(map_ref!) → Full Component Recreation

// Performance problems from signal multiplication
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
- Moved directly to MoonZoon when battle-tested
- Used by any MoonZoon application

```rust
// frontend/src/reactive_actors/mod.rs
// OR as a separate crate: zoon-actors

pub mod relay;
pub mod actor;
pub mod actor_vec;
pub mod actor_map;
pub mod testing;

// Re-export main types
pub use relay::Relay;
pub use actor::Actor;
pub use actor_vec::ActorVec;
pub use actor_map::ActorMap;

// Testing utilities
pub use testing::{MockRelay, TestActor, ActorTestHarness};

// Convenience function for creating Relay+Stream pairs
pub fn relay<T>() -> (Relay<T>, impl Stream<Item = T>) {
    let relay = Relay::new();
    let stream = relay.subscribe();
    (relay, stream)
}
```

### Dependencies
```toml
[dependencies]
futures = "0.3"
tokio = "1.0"
zoon = { git = "...", rev = "..." }  # or version when released
```

## Architecture Overview

### Core Components

#### **Relay**: Type-safe Event Streaming
- Replaces lossy Signals with reliable message passing
- Typed messages ensure compile-time safety
- Multiple subscribers can listen to events
- Drops events when no listeners (efficiency)

#### **Actor**: Controlled State Management
- Owns a `Mutable<T>` and controls all mutations
- Processes events from Relays sequentially
- Provides reactive signals for UI binding
- Built-in debug tracing and connection tracking

### Data Flow Diagram

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

### Core API Pattern

```rust
/// Creates a new Relay with an associated stream, following Rust's channel pattern.
/// This is the idiomatic way to create a Relay for use with Actors.
pub fn relay<T>() -> (Relay<T>, impl Stream<Item = T>) {
    let relay = Relay::new();
    let stream = relay.subscribe();
    (relay, stream)
}
```

### Basic Actor Pattern

```rust
use futures::select;

// Create event relay and stream
let (increment_relay, mut increment_stream) = relay();
let (decrement_relay, mut decrement_stream) = relay();

// Create Actor that processes events sequentially
let counter = Actor::new(0, async move |state| {
    loop {
        select! {
            Some(amount) = increment_stream.next() => {
                state.update(|current| current + amount);
            }
            Some(amount) = decrement_stream.next() => {
                state.update(|current| current.saturating_sub(amount));
            }
        }
    }
});

// UI can emit events safely
increment_relay.send(5);
decrement_relay.send(2);

// UI binds to reactive signals
counter.signal()  // Always returns current state reactively
```

### Multi-Stream Processing

```rust
use futures::{select, join};

let (event_a, stream_a) = relay();
let (event_b, stream_b) = relay();
let (event_c, stream_c) = relay();

let actor = Actor::new(State::default(), async move |state| {
    // Option 1: Select for different event types
    loop {
        select! {
            Some(data) = stream_a.next() => state.handle_a(data),
            Some(data) = stream_b.next() => state.handle_b(data),
            Some(data) = stream_c.next() => state.handle_c(data),
        }
    }
    
    // Option 2: Join for coordinated processing
    let (result_a, result_b) = join!(
        stream_a.collect::<Vec<_>>(),
        stream_b.collect::<Vec<_>>()
    );
    state.handle_batch(result_a, result_b);
});
```

### Collection Patterns

```rust
// ActorVec for reactive collections
let (add_item, mut add_stream) = relay();
let (remove_item, mut remove_stream) = relay();

let items = ActorVec::new(vec![], async move |items_vec| {
    loop {
        select! {
            Some(item) = add_stream.next() => {
                items_vec.lock_mut().push_cloned(item);
            }
            Some(index) = remove_stream.next() => {
                if index < items_vec.lock_ref().len() {
                    items_vec.lock_mut().remove(index);
                }
            }
        }
    }
});

// Reactive UI updates
items.signal_vec_cloned()  // Emits VecDiff for efficient updates
```

## Atom Helper

### ⚠️ CRITICAL RULE: NO RAW MUTABLES ⚠️

**NEVER use raw `Mutable<T>` directly in Actor+Relay architecture!**

The entire purpose of this architecture is to eliminate uncontrolled state mutations. **ALL state must be managed through Actors.**

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

### ONLY EXCEPTION: Atom Helper
The `Atom` helper is acceptable for truly local UI state (button hover, dropdown open/closed) as it's still a controlled abstraction:

```rust
// ACCEPTABLE: Atom helper for local UI only
let is_hovered = Atom::new(false);
// This is still controlled - wraps Mutable with clean API
```

#### Complete Atom Implementation

Based on practical usage, here's the correct implementation using Actor+Relay internally:

```rust
/// Unified helper for local UI state - uses Actor+Relay architecture internally
#[derive(Clone, Debug)]
pub struct Atom<T: Clone + Send + Sync + 'static> {
    pub value: Actor<T>,
    pub setter: Relay<T>,
}

impl<T: Clone + Send + Sync + 'static> Atom<T> {
    pub fn new(initial: T) -> Self {
        let (setter, mut setter_stream) = relay();
        
        let value = Actor::new(initial, async move |state| {
            while let Some(new_value) = setter_stream.next().await {
                state.set_neq(new_value);
            }
        });
        
        Atom { value, setter }
    }
    
    // Convenient methods that delegate to Actor+Relay
    pub fn set(&self, value: T) {
        self.setter.send(value);
    }
    
    pub fn signal(&self) -> impl Signal<Item = T> {
        self.value.signal()
    }
    
    // No .get() method - all state access must be through signals
    // This prevents race conditions and maintains architectural consistency
}

impl<T: Default + Clone + Send + Sync + 'static> Default for Atom<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}
```

**Why Atom is Acceptable:**
- Uses Actor+Relay internally - maintains architectural principles
- Provides convenient API for simple use cases
- Still prevents race conditions (no `.get()` method)
- Maintains traceability through Actor infrastructure
- Can be tested like any other Actor

## Type Safety

### Message Types
```rust
// Define clear message types for different operations
#[derive(Clone, Debug)]
enum UserAction {
    Login { username: String, password: String },
    Logout,
    UpdateProfile { name: String, email: String },
}

#[derive(Clone, Debug)]
enum SystemEvent {
    NetworkConnected,
    NetworkDisconnected,
    DataSynced,
}

// Type-safe relays prevent message confusion
let user_actions: Relay<UserAction> = Relay::new();
let system_events: Relay<SystemEvent> = Relay::new();
```

### Actor State Types
```rust
// Well-defined state types with clear invariants
#[derive(Clone, Debug)]
struct AppState {
    current_user: Option<User>,
    connection_status: ConnectionStatus,
    sync_progress: f64,  // 0.0 to 1.0
}

impl AppState {
    // Enforce invariants in methods
    pub fn set_sync_progress(&mut self, progress: f64) {
        self.sync_progress = progress.clamp(0.0, 1.0);
    }
}
```

This architecture provides a solid foundation for building scalable, maintainable MoonZoon applications with clear state management and excellent developer experience.