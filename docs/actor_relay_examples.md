# Actor+Relay Implementation Examples

This document provides practical implementation examples for the Actor+Relay architecture, consolidating the best patterns from the comprehensive architecture documentation.

## Table of Contents

1. [Basic Patterns](#basic-patterns)
2. [Counter Examples](#counter-examples)  
3. [Todo App Example](#todo-app-example)
4. [File Manager Example](#file-manager-example)
5. [Advanced Multi-Stream Processing](#advanced-multi-stream-processing)
6. [UI Component Examples](#ui-component-examples)
7. [Testing Patterns](#testing-patterns)
8. [Common Antipatterns](#common-antipatterns)

## Basic Patterns

### Simple Counter with Unit Relays

The most basic Actor+Relay pattern using discrete button-style events:

```rust
use std::collections::HashMap;
use futures::select;

#[derive(Clone, Default)]
struct SimpleCounter {
    pub value: Actor<i32>,
    
    // Unit relays for discrete actions
    pub increment: Relay,        // Relay<()> is default
    pub decrement: Relay,
    pub reset: Relay,
}

impl SimpleCounter {
    fn new() -> Self {
        // Create relays with streams using modern pattern
        let (increment, mut increment_stream) = relay();
        let (decrement, mut decrement_stream) = relay();
        let (reset, mut reset_stream) = relay();
        
        let value = Actor::new(0, async move |state| {
            loop {
                select! {
                    Some(()) = increment_stream.next() => {
                        state.update(|current| current + 1);  // Atomic operation
                    }
                    Some(()) = decrement_stream.next() => {
                        state.update(|current| current.saturating_sub(1)); // Safe decrement
                    }
                    Some(()) = reset_stream.next() => {
                        state.set_neq(0);  // Reset takes priority
                    }
                }
            }
        });
        
        SimpleCounter { value, increment, decrement, reset }
    }
}

// UI usage - just notify!
fn counter_ui(counter: &SimpleCounter) -> impl Element {
    Row::new()
        .item(
            Button::new()
                .label("-")
                .on_press(|| counter.decrement.send(())) // Simple notification
        )
        .item_signal(counter.value.signal().map(|v| v.to_string()))
        .item(
            Button::new()
                .label("+")
                .on_press(|| counter.increment.send(()))
        )
        .item(
            Button::new()
                .label("Reset")
                .on_press(|| counter.reset.send(()))
        )
}
```

### Counter with Parameterized Changes

```rust
#[derive(Clone, Default)]
struct AdvancedCounter {
    pub value: Actor<i32>,
    pub change_by: Relay<i32>,    // Parameterized relay
    pub multiply: Relay<i32>,
    pub reset: Relay,
}

impl AdvancedCounter {
    fn new() -> Self {
        let (change_by, mut change_stream) = relay();
        let (multiply, mut multiply_stream) = relay();
        let (reset, mut reset_stream) = relay();
        
        let value = Actor::new(0, async move |state| {
            loop {
                select! {
                    Some(amount) = change_stream.next() => {
                        state.update(|current| current + amount);
                    }
                    Some(factor) = multiply_stream.next() => {
                        state.update(|current| current * factor);
                    }
                    Some(()) = reset_stream.next() => {
                        state.set_neq(0);
                    }
                }
            }
        });
        
        AdvancedCounter { value, change_by, multiply, reset }
    }
    
    // Convenience methods (optional)
    pub fn increment(&self) {
        self.change_by.send(1);
    }
    
    pub fn decrement(&self) {
        self.change_by.send(-1);
    }
}
```

## Counter Examples

### App-Level Counter with Struct Methods

The most idiomatic Rust approach using struct methods:

```rust
#[derive(Clone, Default)]
struct CounterApp {
    // Flattened state - no unnecessary wrapper structs
    value: Actor<i32>,
    change_by: Relay<i32>,
    ui_state: SimpleState<bool>,
}

/// Helper for local UI state that doesn't need Actor overhead
/// Replaces raw Mutable usage with simpler pattern
#[derive(Clone, Debug)]
struct SimpleState<T: Clone> {
    state: Mutable<T>,
}

impl<T: Clone> SimpleState<T> {
    fn new(initial: T) -> Self {
        Self { state: Mutable::new(initial) }
    }
    
    fn set(&self, value: T) {
        self.state.set_neq(value);
    }
    
    fn signal(&self) -> impl Signal<Item = T> {
        self.state.signal()
    }
}

impl CounterApp {
    fn new() -> Self {
        let (change_by, mut change_stream) = relay();
        
        let value = Actor::new(0, async move |state| {
            while let Some(amount) = change_stream.next().await {
                state.update(|current| current + amount);
            }
        });
        
        CounterApp {
            value,
            change_by,
            ui_state: SimpleState::new(false),
        }
    }
    
    // Clean method structure - no parameter passing chaos
    fn root(&self) -> impl Element {
        Column::new()
            .item(self.counter_controls())  // self method call
            .item(self.status_panel())      // self method call
    }
    
    fn counter_controls(&self) -> impl Element {
        Row::new()
            .item(self.counter_button("-", -1))  // Passing primitives
            .item_signal(self.value.signal().map(|v| v.to_string()))
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
    
    fn status_panel(&self) -> impl Element {
        El::new()
            .child_signal(self.ui_state.signal().map(|visible| {
                if visible {
                    Text::new("Counter is active").into_element()
                } else {
                    Text::new("Counter is idle").into_element()
                }
            }))
    }
}

// Perfect lifetime handling - works because Actors are Arc internally
fn main() {
    start_app("app", || CounterApp::new().root());
}
```

## Todo App Example

### Structural Pattern with Clean Domain Model

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

## File Manager Example

### Clean Architecture with File States

```rust
use std::path::PathBuf;

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

## Advanced Multi-Stream Processing

### Data Processor with Configuration and Events

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
    
    // Public API for sending events
    pub fn receive_data(&self, id: String, data: Vec<u8>) {
        self.data_events.send(DataEvent::ItemReceived { id, data });
    }
    
    pub fn complete_batch(&self, batch_id: String) {
        self.data_events.send(DataEvent::BatchComplete { batch_id });
    }
    
    pub fn update_status(&self, message: String) {
        self.status_updates.send(message);
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

## UI Component Examples

### User Interface with Discrete Actions

```rust
#[derive(Clone, Debug)]
enum AppMode {
    Normal,
    Saving,
    Loading,
    Exiting,
    ShowingHelp,
}

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
async fn perform_save() { /* save logic */ }
async fn perform_load() { /* load logic */ }
async fn cleanup() { /* cleanup logic */ }

fn ui_controls(ui: &UserInterface) -> impl Element {
    Row::new()
        .item(
            Button::new()
                .label("Save")
                .on_press(|| ui.save_clicked.send(()))  // Simple notification
        )
        .item(
            Button::new()
                .label("Load")
                .on_press(|| ui.load_clicked.send(()))
        )
        .item(
            Button::new()
                .label("Help")
                .on_press(|| ui.help_clicked.send(()))
        )
        .item(
            Button::new()
                .label("Exit")
                .on_press(|| ui.exit_clicked.send(()))
        )
}
```

## Testing Patterns

### Signal-Based Reactive Testing

All examples consistently use this pattern:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_counter_increment() {
        let counter = SimpleCounter::new();
        
        // Send event through relay
        counter.increment.send(());
        
        // ✅ CORRECT: Use signal.await to get value reactively
        let final_value = counter.value.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, 1);
    }
    
    #[async_test]  
    async fn test_multiple_operations() {
        let counter = AdvancedCounter::new();
        
        // Test parameterized changes
        counter.change_by.send(5);
        let value_after_add = counter.value.signal().to_stream().next().await.unwrap();
        assert_eq!(value_after_add, 5);
        
        // Test multiplication
        counter.multiply.send(3);
        let value_after_multiply = counter.value.signal().to_stream().next().await.unwrap();
        assert_eq!(value_after_multiply, 15);
        
        // Test reset
        counter.reset.send(());
        let value_after_reset = counter.value.signal().to_stream().next().await.unwrap();
        assert_eq!(value_after_reset, 0);
    }
    
    #[async_test]
    async fn test_todo_operations() {
        let todo = Todo::new(Uuid::new_v4(), "Test task".to_string());
        
        // Test completion toggle
        todo.clicked.send(());
        let completed = todo.completed.signal().to_stream().next().await.unwrap();
        assert!(completed);
        
        // Test text change
        todo.text_changed.send("Updated task".to_string());
        let updated_text = todo.text.signal().to_stream().next().await.unwrap();
        assert_eq!(updated_text, "Updated task");
    }
}
```

### Mock Testing with TaskHandle Management

```rust
#[cfg(test)]
mod test_utils {
    use super::*;
    
    /// Test harness for proper task cleanup
    pub struct TestHarness {
        tasks: Vec<TaskHandle<()>>,
    }
    
    impl TestHarness {
        pub fn new() -> Self {
            Self { tasks: Vec::new() }
        }
        
        pub fn spawn_task<F>(&mut self, future: F) -> TaskHandle<()>
        where
            F: Future<Output = ()> + 'static,
        {
            let handle = Task::start_droppable(future);
            self.tasks.push(handle.clone());
            handle
        }
        
        pub async fn wait_for_processing(&self, timeout_ms: u64) {
            Timer::sleep(timeout_ms).await;
        }
    }
    
    impl Drop for TestHarness {
        fn drop(&mut self) {
            // All tasks automatically cancelled when handles drop
            self.tasks.clear();
        }
    }
}
```

## Common Antipatterns

### ❌ NEVER DO: clone! Macro Instead of relay()

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

### ❌ NEVER DO: Multiple relay.send() from Same Location

```rust
// ❌ WRONG: Will panic with "multiple source" error
fn test() {
    relay.send(()); // First source location  
    relay.send(()); // PANIC: Multiple source locations detected
}

// ✅ CORRECT: Use Task::start for different call sites
for _ in 0..3 {
    Task::start(async { relay.send(()); });
}

// ✅ CORRECT: Batch relay with count parameter
batch_relay.send(3); // Send count instead of multiple individual sends
```

### ❌ NEVER DO: .get() for Read-Modify-Write Operations

```rust
// ❌ RACE CONDITION: Value can change between get() and set()
let current = state.get();  // This method doesn't exist in Actor API
state.set_neq(current + amount);

// ✅ ATOMIC: Single operation, no races possible
state.update(|value| value + amount);
```

### ❌ NEVER DO: Task::start_droppable in Actor Setup

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

### ❌ AVOID: Raw Mutable Instead of SimpleState

```rust
// ❌ INCONSISTENT: Raw Mutable scattered throughout
static DIALOG_STATE: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// ✅ UNIFIED: SimpleState for consistent local state management
struct UIState {
    dialog_open: SimpleState<bool>,
    filter_text: SimpleState<String>,
}
```

## Key Benefits Summary

**🎯 No Custom Message Types**: Events are just notifications, data lives in Actors
**🔧 Compile-time Safety**: Relay identity comes from struct field names  
**⚡ Pre-wired Logic**: Business logic connected during creation, UI just hooks up
**📝 Clear Separation**: Creation (business logic) vs UI (event connection)
**🚀 Simple**: `Relay` defaults to `Relay<()>` for most use cases
**🏗️ Idiomatic Rust**: Struct methods with `self` feel natural to Rust developers
**🔒 Race-Free**: Atomic operations prevent concurrent access issues
**🧪 Testable**: Signal-based testing with proper async patterns

This approach eliminates the explosion of typed message structs while maintaining full type safety and clear data flow.