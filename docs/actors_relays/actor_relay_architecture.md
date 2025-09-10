# Actor+Relay Architecture: A Simple Guide

**Why Actor+Relay is better than just Mutables and Signals**

If you're familiar with MoonZoon's `Mutable<T>` and signals, you might wonder: why do we need something more complex? This guide explains the problems with raw Mutables and how Actor+Relay solves them.

## The Problem with Raw Mutables

### What are Mutables?
In MoonZone, `Mutable<T>` is reactive state - when you change it, UI updates automatically:

```rust
// Simple reactive state
let counter = Mutable::new(0);

// UI updates when value changes
Text::new_signal(counter.signal().map(|n| n.to_string()))

// Change triggers UI update
counter.set(42); // UI shows "42"
```

### Why Raw Mutables Become Problematic

**1. Global State Chaos**
```rust
// ❌ PROBLEMATIC: Global mutables everywhere
static USERNAME: Lazy<Mutable<String>> = Lazy::new(|| Mutable::new("".to_string()));
static MESSAGES: Lazy<MutableVec<Message>> = lazy::default();
static IS_CONNECTED: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
static ERROR_MESSAGE: Lazy<Mutable<String>> = lazy::default();

// Anyone can change anything from anywhere!
USERNAME.set("Alice".to_string());        // From login.rs
IS_CONNECTED.set(false);                  // From network.rs  
MESSAGES.lock_mut().clear();              // From chat.rs
```

**Problems:**
- **Who changed what?** Hard to debug when state changes unexpectedly
- **Tangled dependencies** - One global change can break unrelated features
- **Testing nightmare** - Global state makes unit tests interfere with each other
- **Race conditions** - Multiple parts changing the same state simultaneously

**2. Mixed Concerns**
```rust
// ❌ PROBLEMATIC: Business logic mixed with UI state
fn send_message() {
    let message = NEW_MESSAGE_TEXT.take(); // Get input text
    if !message.is_empty() {
        MESSAGES.lock_mut().push_cloned(Message {
            text: message,
            username: USERNAME.get_cloned(), // Cross-cutting concern
        });
        SCROLL_POSITION.set(i32::MAX);      // UI concern
        CONNECTION.send(message);           // Network concern
    }
}
```

**3. No Control or Validation**
```rust
// ❌ PROBLEMATIC: No validation or business rules
COUNTER.set(-100);              // Counter goes negative?
USERNAME.set("");               // Empty username allowed?
MESSAGES.lock_mut().clear();    // Messages lost accidentally?
```

**4. Deadlock Hell and WASM Threading Issues**
```rust
// ❌ PROBLEMATIC: Nested locks cause deadlocks
fn update_user_stats() {
    let username = USERNAME.lock_ref().clone();  // Lock 1
    let mut messages = MESSAGES.lock_mut();      // Lock 2
    
    // Another function tries to access in reverse order = DEADLOCK!
    messages.iter().for_each(|msg| {
        if msg.username == username {
            COUNTER.lock_mut().increment();     // Lock 3 - potential deadlock!
        }
    });
}

// ❌ WASM THREADING PROBLEM: Main thread blocks entire module
fn background_processing() {
    zoon::Task::start(async {
        let data = LARGE_DATA.lock_mut();        // Blocks on background thread
        // Main thread cannot continue - entire WASM module freezes!
        process_large_data(data).await;
    });
}
```

**Problems with locks:**
- **Deadlock debugging nightmare** - Hard to trace which locks are held where
- **WASM single-threaded reality** - Background thread locks block main thread completely
- **Lock ordering requirements** - Must acquire locks in same order everywhere
- **Hidden dependencies** - No clear picture of what data depends on what

## How Actor+Relay Fixes These Problems

### Core Idea: Controlled State with Event Streams

Instead of allowing anyone to change state directly, Actor+Relay works like this:

1. **Actor** = owns and controls a piece of state
2. **Relay** = typed event stream that tells the Actor what happened
3. **Actor processes events** and decides how to update state

```rust
// ✅ BETTER: Controlled state with events
struct Counter {
    value: Actor<i32>,           // State controlled by Actor
    increment: Relay,            // Event: "increment button clicked"
    decrement: Relay,            // Event: "decrement button clicked"  
    reset: Relay,                // Event: "reset button clicked"
}
```

### Example: From Global Chaos to Clean Structure

**❌ Before: Global Mutables**
```rust
use zoon::*;

// Global state anyone can modify
static COUNTER: Lazy<Mutable<i32>> = Lazy::new(|| Mutable::new(0));

// UI directly modifies global state
fn increment_button() -> impl Element {
    button()
        .on_press(|| {
            let current = COUNTER.get();
            COUNTER.set(current + 1);        // Direct mutation
        })
        .label("+1")
        .build()
}
```

** After: Actor+Relay**
```rust
use zoon::*;
use futures::{select, stream::StreamExt, FutureExt};
use crate::dataflow::{Actor, Relay, relay};

#[derive(Clone)]
struct CounterApp {
    value: Actor<i32>,                    // State owned by Actor
    increment: Relay,                     // Events describe what happened
    decrement: Relay,
}

impl Default for CounterApp {
    fn default() -> Self {
        // Create event streams
        let (increment, mut increment_stream) = relay();
        let (decrement, mut decrement_stream) = relay();
        
        // Actor processes events and controls state
        let value = Actor::new(0, async move |state_handle| {
            loop {
                select! {
                    Some(()) = increment_stream.next() => {
                        state_handle.update(|current| *current += 1); // Controlled update
                    }
                    Some(()) = decrement_stream.next() => {
                        state_handle.update(|current| *current = current.saturating_sub(1)); // With validation!
                    }
                }
            }
        });
        
        CounterApp { value, increment, decrement }
    }
}

// UI emits events, doesn't modify state directly
impl CounterApp {
    fn increment_button(&self) -> impl Element {
        button()
            .on_press({
                let increment_relay = self.increment.clone();
                move || increment_relay.send(()) // Send event, don't modify state
            })
            .label("+1")
            .build()
    }
}
```

## Key Benefits Explained

### 1. **Single Source of Truth**
```rust
//  Only the Actor can change its state
let value = Actor::new(0, async move |state| {
    // This is the ONLY place where counter value changes
    // All changes go through this controlled process
});

// UI sends events, doesn't change state directly
increment_relay.send(());  // "Hey, increment button was clicked"
```

### 2. **Event-Driven Architecture**
```rust
// Events describe WHAT HAPPENED, not what to do
file_dropped_relay: Relay<Vec<PathBuf>>,     // "Files were dropped"
user_clicked_save_relay: Relay,              // "User clicked save"
login_succeeded_relay: Relay<Username>,      // "Login succeeded"

// ❌ NOT commands
add_file_relay: Relay<PathBuf>,              // Sounds like "add this file"
set_username_relay: Relay<String>,           // Sounds like "set username to this"
```

### 3. **Business Logic in One Place**
```rust
//  All business rules in the Actor
let counter = Actor::new(0, async move |state| {
    loop {
        select! {
            Some(()) = increment_stream.next() => {
                state.update(|current| {
                    if *current < 100 {                    // Business rule: max 100
                        *current += 1;
                    } else {
                        error_relay.send("Counter at maximum");
                    }
                });
            }
            Some(()) = reset_stream.next() => {
                state.set(0);                        // Reset always allowed
                reset_completed_relay.send(());      // Notify UI
            }
        }
    }
});
```

### 4. **No More Deadlocks or Threading Issues**
```rust
// ✅ SOLUTION: Sequential processing eliminates deadlocks
let user_stats_actor = Actor::new(UserStats::default(), async move |state_handle| {
    loop {
        select! {
            Some(new_message) = message_stream.next() => {
                // Process one event at a time - no nested locks!
                state_handle.update(|stats| {
                    stats.message_count += 1;
                    if new_message.username == "Alice" {
                        stats.alice_messages += 1;
                    }
                });
            }
            Some(username_change) = username_change_stream.next() => {
                // Events processed sequentially - no lock conflicts!
                state_handle.update(|stats| {
                    stats.last_username_change = username_change;
                });
            }
        }
    }
});

// ✅ WASM-FRIENDLY: No blocking locks
fn background_processing_actor() -> Actor<ProcessingState> {
    Actor::new(ProcessingState::Idle, async move |state_handle| {
        loop {
            select! {
                Some(data) = large_data_stream.next() => {
                    // State changes are atomic and non-blocking
                    state_handle.set(ProcessingState::Processing);
                    
                    // Process without holding locks
                    let result = process_data(data).await;
                    
                    state_handle.set(ProcessingState::Complete(result));
                }
            }
        }
    })
}
```

**Why this eliminates lock problems:**
- **Sequential processing** - Events processed one at a time, never simultaneously
- **No nested locks** - Actor owns its state exclusively, no lock conflicts possible  
- **WASM compatible** - No blocking locks that can freeze the main thread
- **Clear data flow** - Event → Actor → State change, easy to trace and debug

### 5. **Event Waiting for Easy Synchronization**
```rust
// ✅ SOLUTION: Wait for events naturally with async/await
let chat_actor = Actor::new(ChatState::default(), async move |state_handle| {
    loop {
        select! {
            Some(username) = username_changed_stream.next() => {
                state_handle.update(|state| state.username = username);
            }
            Some(message_text) = message_changed_stream.next() => {
                state_handle.update(|state| state.message_text = message_text);
            }
            Some(()) = send_clicked_stream.next() => {
                // Use cached values pattern for coordination
                state_handle.update(|state| {
                    if !state.username.is_empty() && !state.message_text.is_empty() {
                        let username = state.username.clone();
                        let message = state.message_text.clone();
                        
                        // Clear message after copying
                        state.message_text.clear();
                        
                        // Send message asynchronously (would need task spawn)
                        send_message_relay.send((username, message));
                    }
                });
            }
        }
    }
});

// ✅ EASY COORDINATION: Wait for multiple events
async fn complex_workflow() {
    // Wait for user to be logged in
    user_logged_in_stream.next().await;
    
    // Wait for config to load
    config_loaded_stream.next().await;
    
    // Wait for initial data fetch
    data_fetched_stream.next().await;
    
    // Now everything is ready - start main app
    app_ready_relay.send(());
}
```

**Compare with callback hell:**
```rust
// ❌ CALLBACK NIGHTMARE: Traditional callback-based approach
fn setup_callbacks() {
    set_username_callback(|username| {
        if message_ready() && username_ready() {
            // Need to coordinate multiple callbacks
            maybe_send_message(); // Complex state tracking needed
        }
    });
    
    set_message_callback(|message| {
        if username_ready() && message_ready() {
            // Duplicate coordination logic
            maybe_send_message(); // Same logic scattered everywhere
        }
    });
    
    set_send_callback(|| {
        // No easy way to wait for prerequisites
        if all_conditions_met() { // Manual state checking
            send_message();
        }
    });
}
```

**Why event waiting is superior:**
- **Natural async/await** - Use standard Rust async patterns instead of callback coordination
- **Sequential logic** - Write code that reads top-to-bottom instead of scattered callbacks
- **Easy coordination** - Wait for multiple events with simple `.next().await` calls
- **No callback hell** - Avoid deeply nested callbacks and complex state tracking

### 6. **Non-Blocking DOM Event Handlers**

**✅ SOLUTION: Relays never block DOM event processing**
```rust
// ✅ RELAY EVENTS: Instant DOM response, async processing
button()
    .on_press({
        let file_process_relay = app.file_process_relay.clone();
        move || {
            file_process_relay.send(large_file_path.clone()); // Instant return!
            // DOM immediately ready for next user interaction
        }
    })
    .build()

// Heavy processing happens in Actor - DOM stays responsive
let file_actor = Actor::new(FileState::default(), async move |state_handle| {
    loop {
        select! {
            Some(file_path) = file_process_stream.next() => {
                // Heavy file processing doesn't block DOM
                let processed_data = heavy_file_processing(file_path).await;
                state_handle.update(|state| state.files.push(processed_data));
            }
        }
    }
});
```

**❌ COMPARISON: Traditional direct processing blocks DOM**
```rust
// ❌ DIRECT PROCESSING: DOM freezes during heavy operations
button.on_click(|| {
    let result = heavy_file_processing(file_path); // DOM blocked!
    // User cannot click other buttons, scroll, or interact
    update_ui_with_result(result);
});

// ❌ EVEN ASYNC CLOSURES: Still block if not properly yielded
button.on_click(|| async {
    let result = heavy_processing().await; // Blocks until complete
    update_ui(result);
});
```

**Why relay-based events are superior:**
- **Instant DOM response** - Event handlers return immediately, keeping UI responsive
- **Background processing** - Heavy work happens in Actors without blocking user interactions
- **Natural async boundaries** - Proper separation between UI events and business logic
- **No UI freezing** - Users can continue interacting while operations complete
- **Better UX** - Responsive interface even during intensive operations

### 7. **Built-in Debugging with Source Location Tracking**

**✅ SOLUTION: Automatic source location tracking for relay events**
```rust
// ✅ RELAY DEBUG: Automatically tracks where events are sent from
#[track_caller]  // Built into relay implementation
pub fn send(&self, value: T) {
    let location = std::panic::Location::caller();
    println!("🔍 Event sent from {}:{}", location.file(), location.line());
    // Send event with location context
}

// Debug output automatically shows source:
// 🔍 Event sent from src/file_panel.rs:42
// 🔍 Event sent from src/toolbar.rs:156
```

**❌ COMPARISON: Manual debugging with global state**
```rust
// ❌ MUTABLE DEBUG: Manual tracking required everywhere
static FILES: Lazy<MutableVec<File>> = lazy::default();

fn add_file(file: File) {
    println!("🐛 Adding file from... where?"); // No automatic location
    FILES.lock_mut().push_cloned(file);
    // Need to manually add debug info at every call site
}
```

**Debug benefits:**
- **Automatic location tracking** - No manual debug logging needed
- **Event source identification** - Instantly know which component sent what
- **Call stack context** - Full trace of event flow through the system
- **Zero overhead** - Location tracking only in debug builds

### 8. **Custom Debouncing and Throttling**

**✅ SOLUTION: Custom debounce patterns with Actors**
```rust
// ✅ CONFIG SAVE DEBOUNCE: Automatic batching of rapid changes
struct ConfigSaver {
    save_requested_relay: Relay,
    debounce_actor: Actor<()>, // Keep actor alive
}

impl ConfigSaver {
    fn new() -> Self {
        let (save_requested_relay, mut save_stream) = relay();
        
        // Debounce actor with Timer::sleep cancellation
        let debounce_actor = Actor::new((), async move |_state| {
            loop {
                select! {
                    _ = save_stream.next() => {
                        // Wait for quiet period, cancelling if new event arrives
                        loop {
                            select! {
                                // New save request cancels timer
                                _ = save_stream.next() => {
                                    continue; // Restart timer
                                }
                                // Timer completes - do the save
                                _ = Timer::sleep(500).fuse() => {
                                    save_config_to_disk().await;
                                    println!("💾 Config saved after debounce");
                                    break; // Back to outer loop
                                }
                            }
                        }
                    }
                }
            }
        });
        
        Self { save_requested_relay, debounce_actor }
    }
}

// Usage: Rapid panel resizing only saves once
let config_saver = ConfigSaver::new();

// Panel resize handler - can fire 60+ times per second
panel.on_resize({
    let save_relay = config_saver.save_requested_relay.clone();
    move |new_size| {
        update_panel_size(new_size);
        save_relay.send(()); // Request save (debounced automatically)
    }
})
```

**❌ COMPARISON: Manual debounce with global state**
```rust
// ❌ MANUAL DEBOUNCE: Complex state management scattered everywhere
static SAVE_PENDING: Lazy<Mutable<bool>> = lazy::default();
static LAST_RESIZE_TIME: Lazy<Mutable<u64>> = lazy::default();

fn on_panel_resize(new_size: f32) {
    update_panel_size(new_size);
    
    // Manual debounce logic repeated everywhere
    let now = current_time_ms();
    LAST_RESIZE_TIME.set(now);
    
    if !SAVE_PENDING.get() {
        SAVE_PENDING.set(true);
        // Complex timer coordination needed everywhere
        spawn_save_timer(); // Race conditions possible!
    }
}
```

**Note:** This example shows how to write customized debounce logic using Actors and select! patterns. While it requires a few lines of code per use case, it provides complete control over the debouncing behavior. A generic debounce Actor or similar API may be added in the future to simplify common patterns.

**Debounce/throttling benefits:**
- **Automatic batching** - Multiple rapid events become single action
- **Performance optimization** - Prevents excessive file I/O or network calls
- **Custom logic** - Write exactly the debounce behavior you need
- **No race conditions** - Actor ensures sequential processing
- **Reusable pattern** - Same approach works for any event type

### 9. **Easy Testing**
```rust
#[tokio::test]
async fn test_counter() {
    let counter = CounterApp::default();
    let mut value_stream = counter.value.signal().to_stream();
    
    // Test initial value
    assert_eq!(value_stream.next().await.unwrap(), 0);
    
    // Send events and check results
    counter.increment.send(());
    assert_eq!(value_stream.next().await.unwrap(), 1);
    
    counter.decrement.send(());
    assert_eq!(value_stream.next().await.unwrap(), 0);
}
```

## The "Cache Current Values" Pattern

Sometimes you need multiple values at once when handling an event. This is the ONLY place where you can cache values:

```rust
// ✅ CORRECT: Cache values ONLY inside Actor loops
let chat = Actor::new(vec![], async move |messages| {
    // Cache current form values
    let mut current_username = String::new();
    let mut current_message = String::new();
    
    loop {
        select! {
            // Update cached values when they change
            Some(username) = username_stream.next() => {
                current_username = username;
            }
            Some(message) = message_stream.next() => {
                current_message = message;
            }
            
            // Use cached values when send button clicked
            Some(()) = send_button_stream.next() => {
                if !current_message.trim().is_empty() {
                    let chat_message = ChatMessage {
                        username: current_username.clone(),
                        text: current_message.clone(),
                    };
                    messages.lock_mut().push_cloned(chat_message);
                    current_message.clear(); // Clear after sending
                }
            }
        }
    }
});
```

**Why this works:** You're caching values inside the Actor that controls the state, so there's still only one place that manages the data.

## Real-World Example: Chat App

**L Before: Messy global state**
```rust
static USERNAME: Lazy<Mutable<String>> = Lazy::new(|| Mutable::new("".to_string()));
static MESSAGES: Lazy<MutableVec<Message>> = lazy::default();
static NEW_MESSAGE: Lazy<Mutable<String>> = lazy::default();

fn send_message() {
    let username = USERNAME.get_cloned();    // Global access
    let message = NEW_MESSAGE.take();        // Global mutation
    MESSAGES.lock_mut().push_cloned(Message { username, text: message }); // Global mutation
}
```

** After: Clean Actor+Relay structure**
```rust
#[derive(Clone)]
struct ChatApp {
    // State managed by Actors
    messages: ActorVec<Message>,
    username: Actor<String>,
    message_text: Actor<String>,
    
    // Events describe what happened
    send_clicked: Relay,
    username_changed: Relay<String>,
    message_changed: Relay<String>,
}

impl ChatApp {
    fn send_button(&self) -> impl Element {
        button()
            .on_press({
                let send_relay = self.send_clicked.clone();
                move || send_relay.send(()) // Just emit event
            })
            .label("Send")
            .build()
    }
}
```

## When to Use What

### Use **Actor+Relay** for:
- Business logic and domain state
- Data shared between components  
- State that needs validation or business rules
- State that should be testable

### Use **Atom** (simple helper) for:
- Local UI state (button hover, dialog open/closed)
- Simple toggles without business logic
- Component-specific state

```rust
//  Atom for simple UI state
let is_hovered = Atom::new(false);
let dialog_open = Atom::new(false);

//  Actor+Relay for business state  
struct UserProfile {
    name: Actor<String>,
    email: Actor<String>,
    name_changed: Relay<String>,
    email_changed: Relay<String>,
}
```

## Summary

**Actor+Relay vs Raw Mutables:**

| Raw Mutables | Actor+Relay |
|-------------|-------------|
| Anyone can change anything | Only Actor controls its state |
| No validation or business rules | Business logic centralized in Actor |
| Hard to test (global state) | Easy to test (send events, check results) |
| Debugging nightmare | Clear event trail |
| Mixed concerns everywhere | Clean separation of concerns |
| Race conditions possible | Sequential event processing |
| **Deadlock-prone nested locks** | **No deadlocks - single Actor owns state** |
| **WASM threading blocks main thread** | **WASM-friendly non-blocking design** |
| **Callback hell for coordination** | **Natural async/await event waiting** |

The key insight: **Instead of allowing direct state mutation everywhere, use events to describe what happened, and let Actors decide how to respond.**

This makes your code more predictable, testable, and maintainable as it grows.