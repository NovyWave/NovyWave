# Actor+Relay Complete API Reference

This document provides the complete API specification for the Actor+Relay architecture in MoonZoon applications. These designs were extracted from the original architecture documents and represent the full intended API surface.

## Table of Contents

1. [Module Structure](#module-structure)
2. [Core Function: relay()](#core-function-relay)
3. [Relay<T> API](#relayt-api)
4. [Actor<T> API](#actort-api)
5. [ActorVec<T> API](#actorvect-api)
6. [ActorMap<K,V> API](#actormapkv-api)
7. [Atom<T> Helper API](#atomt-helper-api)
8. [Critical Pattern: Cache Current Values](#critical-pattern-cache-current-values)

## Module Structure

The Actor+Relay architecture is organized into two distinct modules:

### `frontend/src/dataflow/` - Core Dataflow Primitives
This module contains the foundational reactive dataflow primitives:
- `relay.rs` - Relay<T> implementation for event broadcasting
- `actor.rs` - Actor<T> for reactive state management
- `actor_vec.rs` - ActorVec<T> for reactive collections
- `actor_map.rs` - ActorMap<K,V> for reactive key-value maps
- `atom.rs` - Atom<T> helper for local UI state
- `mod.rs` - Module exports and the `relay()` function

**Import from dataflow:**
```rust
use crate::dataflow::{Actor, ActorVec, ActorMap, Atom, Relay, relay};
```

### `frontend/src/actors/` - Business Domain Actors
This module contains domain-specific actors built on top of dataflow primitives:
- `tracked_files.rs` - TrackedFiles domain for waveform file management
- `selected_variables.rs` - SelectedVariables domain for variable selection
- `waveform_timeline.rs` - WaveformTimeline domain for timeline state
- `user_configuration.rs` - UserConfiguration domain for app settings
- Other domain modules...

The actors module re-exports dataflow primitives for backward compatibility:
```rust
// In frontend/src/actors/mod.rs
pub use crate::dataflow::{Actor, ActorVec, ActorMap, Atom, Relay, relay, ...};
```

**Import patterns:**
```rust
// Direct from dataflow (recommended for new code)
use crate::dataflow::{Actor, Relay, relay};

// Via actors module (backward compatibility)
use crate::actors::{Actor, Relay, relay};

// Domain actors
use crate::actors::{TrackedFiles, SelectedVariables, WaveformTimeline};
```

## Core Function: relay()

```rust
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
pub fn relay<T>() -> (Relay<T>, impl Stream<Item = T>) 
where 
    T: Clone + Send + Sync + 'static
{
    let relay = Relay::new();
    let stream = relay.subscribe();
    (relay, stream)
}
```

## Relay<T> API

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
    /// Automatically logs any errors to browser console. 
    /// For explicit error handling, use try_send().
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

### Event-Source Naming Convention

All relays MUST follow the `{source}_{event}_relay` naming pattern:

```rust
// ✅ CORRECT: Event-source naming
button_clicked_relay: Relay,              // User clicked button
file_loaded_relay: Relay<PathBuf>,        // File finished loading
input_changed_relay: Relay<String>,       // Input text changed
error_occurred_relay: Relay<String>,      // System error happened

// ❌ PROHIBITED: Command-like naming
add_file: Relay<PathBuf>,                 // Sounds like command
remove_item: Relay<String>,               // Imperative style
set_theme: Relay<Theme>,                  // Action-oriented
```

## Actor<T> API

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
    
    /// Get reference signal for efficient transformations
    /// Allows mapping without cloning the entire state
    pub fn signal_ref<U>(&self, f: impl Fn(&T) -> U + 'static) -> impl Signal<Item = U>
    where
        U: Clone + 'static;
    
    // NO .get() method - all state access must be through signals
    // This prevents race conditions and maintains architectural consistency
    // For testing, use signal-based assertions: signal().to_stream().next().await
}
```

### Usage Example

```rust
let (increment, mut increment_stream) = relay();
let counter = Actor::new(0, async move |state| {
    while let Some(amount) = increment_stream.next().await {
        state.update(|n| n + amount);
    }
});

// Efficient signal transformation without cloning
let doubled = counter.signal_ref(|n| n * 2);
let is_even = counter.signal_ref(|n| n % 2 == 0);
```

## ActorVec<T> API

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
    
    /// Get reactive signal vec for efficient UI updates
    pub fn signal_vec(&self) -> impl SignalVec<Item = T>;
    
    /// Get reactive signal vec with cloning
    pub fn signal_vec_cloned(&self) -> impl SignalVec<Item = T>;
    
    /// Get length signal for reactive count displays
    pub fn len_signal(&self) -> impl Signal<Item = usize>;
}
```

### Usage Example

```rust
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

// Use in UI
Text::new().content_signal(items.len_signal().map(|len| format!("{} items", len)))
```

## ActorMap<K,V> API

```rust
/// Ordered map state management (using BTreeMap internally)
#[derive(Clone, Debug)]
pub struct ActorMap<K, V> 
where 
    K: Ord + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static
{
    map: MutableBTreeMap<K, V>,
    task: Arc<TaskHandle>,
    #[cfg(debug_assertions)]
    creation_location: &'static std::panic::Location<'static>,
}

impl<K, V> ActorMap<K, V>
where 
    K: Ord + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static
{
    /// Create with initial map and async setup
    #[track_caller]
    pub fn new<F>(initial: BTreeMap<K, V>, setup: F) -> Self
    where
        F: for<'a> FnOnce(&'a MutableBTreeMap<K, V>) -> impl Future<Output = ()> + 'a;
    
    /// Get reference signal for efficient map access without cloning
    pub fn signal_map_ref<U>(&self, f: impl Fn(&BTreeMap<K, V>) -> U + 'static) -> impl Signal<Item = U>
    where
        U: Clone + 'static;
    
    /// Get signal for specific key - updates when that key's value changes
    pub fn signal_for_key(&self, key: &K) -> impl Signal<Item = Option<V>>;
    
    /// Get keys as efficient SignalVec (only sends diffs)
    pub fn keys_signal_vec(&self) -> impl SignalVec<Item = K>;
    
    /// Get values as efficient SignalVec (only sends diffs)  
    pub fn values_signal_vec(&self) -> impl SignalVec<Item = V>;
    
    /// Get length signal for reactive size displays
    pub fn len_signal(&self) -> impl Signal<Item = usize>;
    
    // NO .get() method - all access must be through signals
    // Use signal_ref() or signal_map_ref() for reactive access
}
```

### Usage Example

```rust
let (set_value, mut set_stream) = relay();
let (remove_key, mut remove_stream) = relay();

let cache = ActorMap::new(BTreeMap::new(), async move |map| {
    loop {
        select! {
            Some((key, value)) = set_stream.next() => {
                map.lock_mut().insert_cloned(key, value);
            }
            Some(key) = remove_stream.next() => {
                map.lock_mut().remove(&key);
            }
        }
    }
});

// Efficient signal transformations
let total_sum = cache.signal_map_ref(|map| {
    map.values().sum::<i32>()
});

// Watch specific key
let user_score = cache.signal_for_key(&"user123");

// Reactive key/value lists for UI
cache.keys_signal_vec()  // Updates efficiently with only changes
cache.values_signal_vec()  // Only sends diffs, not entire list
```

## Atom<T> Helper API

```rust
/// Unified helper for local UI state - uses Actor+Relay architecture internally
/// 
/// This is the ONLY acceptable abstraction over raw Mutables in Actor+Relay architecture.
/// It maintains all architectural principles while providing a convenient API for simple cases.
#[derive(Clone, Debug)]
pub struct Atom<T: Clone + Send + Sync + 'static> {
    pub value: Actor<T>,
    pub setter: Relay<T>,
}

impl<T: Clone + Send + Sync + 'static> Atom<T> {
    /// Create new Atom with initial value
    pub fn new(initial: T) -> Self {
        let (setter, mut setter_stream) = relay();
        
        let value = Actor::new(initial, async move |state| {
            while let Some(new_value) = setter_stream.next().await {
                state.set_neq(new_value);
            }
        });
        
        Atom { value, setter }
    }
    
    /// Set the value
    pub fn set(&self, value: T) {
        self.setter.send(value);
    }
    
    /// Get reactive signal
    pub fn signal(&self) -> impl Signal<Item = T> {
        self.value.signal()
    }
    
    /// Get reference signal for efficient transformations
    pub fn signal_ref<U>(&self, f: impl Fn(&T) -> U + 'static) -> impl Signal<Item = U>
    where
        U: Clone + 'static
    {
        self.value.signal_ref(f)
    }
    
    // NO .get() method - maintains architectural consistency
}

impl<T: Default + Clone + Send + Sync + 'static> Default for Atom<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}
```

### Usage Example

```rust
// For local UI state only
let dialog_open = Atom::new(false);
let filter_text = Atom::new(String::new());
let selected_index = Atom::new(None::<usize>);

// In UI
button()
    .on_click(move || dialog_open.set(true))
    .label_signal(dialog_open.signal().map(|open| 
        if open { "Close" } else { "Open" }
    ))
```

## Critical Pattern: Cache Current Values

### ⚠️ CRITICAL: This is the ONLY acceptable place to cache values in Actor+Relay architecture

The "Cache Current Values" pattern is used **EXCLUSIVELY inside Actor processing loops** to maintain current state values for use when responding to events.

### The Pattern

```rust
// ✅ CORRECT: Cache values ONLY inside Actors for event response
let messages_actor = ActorVec::new(vec![], async move |messages_vec| {
    // Cache current values as they flow through streams
    let mut current_username = Username::default();
    let mut current_message_text = MessageText::default();
    
    let send_trigger_stream = futures::stream::select(
        enter_pressed_stream,
        send_button_clicked_stream
    );
    
    loop {
        select! {
            // Update cached username when it changes
            Some(username) = username_input_changed_stream.next() => {
                current_username = username;
            }
            
            // Update cached message text when it changes
            Some(text) = message_input_changed_stream.next() => {
                current_message_text = text;
            }
            
            // Use cached values when responding to events
            Some(()) = send_trigger_stream.next() => {
                if !current_message_text.trim().is_empty() {
                    let message = Message { 
                        username: current_username.clone(),
                        text: current_message_text.clone()
                    };
                    // Use the cached values to create and send message
                    connection.send_message(message);
                    
                    // Clear the message text after sending
                    current_message_text = MessageText::default();
                    message_sent_relay.send(());
                }
            }
        }
    }
});
```

### Key Rules

1. **ONLY cache values inside Actor processing loops** - never in UI components
2. **Use caching to get current values when responding to events** (e.g., send button clicked)
3. **Otherwise, always use Actors and Signals** for state management
4. **Never use global variables or raw Mutables for caching** - defeats Actor+Relay architecture

### Why This Pattern Exists

- In Actor loops, you often need multiple values when an event occurs
- Signals are async and can't be queried synchronously inside the loop
- Caching provides synchronous access to current values within the Actor's scope
- This maintains single point of mutation while enabling practical event handling

### Common Use Cases

#### Form Submission
```rust
let form_actor = Actor::new(FormState::default(), async move |state| {
    let mut current_name = String::new();
    let mut current_email = String::new();
    let mut current_message = String::new();
    
    loop {
        select! {
            Some(name) = name_input_stream.next() => {
                current_name = name;
            }
            Some(email) = email_input_stream.next() => {
                current_email = email;
            }
            Some(message) = message_input_stream.next() => {
                current_message = message;
            }
            Some(()) = submit_button_stream.next() => {
                // Use all cached values to submit form
                let form_data = FormData {
                    name: current_name.clone(),
                    email: current_email.clone(),
                    message: current_message.clone(),
                };
                submit_form(form_data).await;
            }
        }
    }
});
```

#### File Dialog Actions
```rust
let dialog_actor = Actor::new(DialogState::default(), async move |state| {
    let mut current_filter = String::new();
    let mut selected_files = Vec::new();
    
    loop {
        select! {
            Some(filter) = filter_input_stream.next() => {
                current_filter = filter;
                // Update filtered view
            }
            Some(files) = file_selection_stream.next() => {
                selected_files = files;
            }
            Some(()) = open_button_stream.next() => {
                // Use cached selection to open files
                for file in &selected_files {
                    open_file(file).await;
                }
            }
        }
    }
});
```

### ❌ NEVER Do This

```rust
// ❌ WRONG: Caching values outside Actor loops
static mut CACHED_USERNAME: String = String::new();  // NEVER!

// ❌ WRONG: Using Mutables for caching
let cached_value = Mutable::new(String::new());  // Defeats architecture!

// ❌ WRONG: Trying to .get() from Actors
let current = some_actor.get();  // No .get() method by design!

// ❌ WRONG: Caching in UI components
fn my_component() -> impl Element {
    let mut cached = String::new();  // NEVER cache in UI!
    // ...
}
```

### Summary

The "Cache Current Values" pattern is a critical part of practical Actor+Relay usage, but it must be used ONLY inside Actor processing loops. This maintains the architectural benefits of single-point mutation and traceability while enabling practical event-driven programming.

**Remember:** If you're not inside an Actor's async processing loop, you should be using signals for all state access!