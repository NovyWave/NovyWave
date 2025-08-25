# Actors and Relays Architecture

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
    /// Returns error in debug builds if called from multiple source locations
    #[track_caller]
    pub fn send(&self, value: T) -> Result<(), RelayError>;
    
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
    
    /// Get current value (clones)
    pub fn get(&self) -> T;
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

All operations that can fail should return `Result`:

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
}
```


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
        // Create relays FIRST
        let clicked = Relay::new();
        let text_changed = Relay::new();  // Relay<String>
        let delete_clicked = Relay::new();
        
        // Clone relays for use in Actor initialization
        let clicked_relay = clicked.clone();
        let text_changed_relay = text_changed.clone();
        
        let completed = Actor::new(false, async move |state| {
            let clicked_relay = clicked_relay.clone();
            // Handle toggle clicks - business logic belongs in the Actor
            clicked_relay.subscribe().for_each(async |_| {
                let current = state.get();
                state.set(!current);  // This is the correct way to update
            }).await;
        });
        
        let text = Actor::new(initial_text, async move |state| {
            let text_changed_relay = text_changed_relay.clone();
            // Handle text changes - business logic in the Actor
            text_changed_relay.subscribe().for_each(async |new_text| {
                state.set(new_text);  // Correct way to update text
            }).await;
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
        // Create relays FIRST
        let add_clicked = Relay::new();
        
        // Clone relays for use in ActorVec initialization
        let add_relay = add_clicked.clone();
        
        let todos = ActorVec::new(vec![], async move |todos_vec| {
            let add_relay = add_relay.clone();
            // Handle todo management here - business logic in the ActorVec
            add_relay.subscribe().for_each(async |_| {
                let new_todo = Todo::new(Uuid::new_v4(), "New todo".to_string());
                todos_vec.lock_mut().push_cloned(new_todo);
            }).await;
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
        // Create relays FIRST so they can be used in Actor initialization
        let remove_clicked = Relay::new();
        let reload_clicked = Relay::new(); 
        let state_changed = Relay::new();
        
        // Clone relays for use in Actor initialization
        let reload_relay = reload_clicked.clone();
        let path_clone = path.clone();
        
        let state = Actor::new(FileState::Loading, async move |state_actor| {
            let reload_relay = reload_relay.clone();
            let path = path_clone.clone();
            
            // Business logic: initial load
            // let result = load_file(&path).await;
            // state_actor.set(match result { Ok(_) => FileState::Ready, Err(e) => FileState::Error(e) });
            
            // Handle reload events - business logic belongs HERE in the Actor
            reload_relay.subscribe().for_each(async |_| {
                state_actor.set(FileState::Loading);
                // let result = load_file(&path).await;
                // state_actor.set(match result { ... });
            }).await;
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
        // Create relays FIRST
        let add_file_clicked = Relay::new();
        let clear_all_clicked = Relay::new();
        
        // Clone relays for use in ActorVec initialization
        let clear_relay = clear_all_clicked.clone();
        
        let files = ActorVec::new(vec![], async move |files_vec| {
            let clear_relay = clear_relay.clone();
            // Collection business logic belongs HERE in the ActorVec
            clear_relay.subscribe().for_each(async |_| {
                // Clear all files logic
                files_vec.lock_mut().clear();
            }).await;
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

### Example 3: Mixed Sources - Combining Actor Signals and Relay Streams

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

/// Bundle related Relays together to reduce cloning boilerplate
#[derive(Clone, Debug)]
struct ProcessorRelays {
    data_events: Relay<DataEvent>,
    status_updates: Relay<String>,
    results_ready: Relay<Vec<ProcessedItem>>,
    error_occurred: Relay<String>,
}

impl ProcessorRelays {
    fn new() -> Self {
        Self {
            data_events: Relay::new(),
            status_updates: Relay::new(),
            results_ready: Relay::new(),
            error_occurred: Relay::new(),
        }
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
    
    // Grouped Relays (cleaner than individual fields)
    relays: ProcessorRelays,
    
    // Task handles for proper cleanup (REQUIRED in Zoon)
    data_task: Mutable<Option<TaskHandle>>,
    status_task: Mutable<Option<TaskHandle>>,
}

impl DataProcessor {
    pub fn new(initial_config: ProcessingConfig) -> Self {
        // Create RelayBundle FIRST - single clone point
        let relays = ProcessorRelays::new();
        
        // Create config Actor SECOND - use clone! macro for cleaner cloning  
        let config = Actor::new(initial_config, clone!((relays) async move |config_state| {
            // Config Actor can respond to external changes
            // Example: Disable processing when too many errors
            relays.data_events.subscribe().for_each(async move |event| {
                if let DataEvent::ErrorOccurred { .. } = event {
                    let current = config_state.get();
                    if current.enabled {
                        // Temporarily disable processing after error
                        let mut new_config = current;
                        new_config.enabled = false;
                        config_state.set(new_config);
                        
                        // Auto re-enable after timeout
                        Timer::sleep(current.timeout_ms).await;
                        let mut restored_config = config_state.get();
                        restored_config.enabled = true;
                        config_state.set(restored_config);
                    }
                }
            }).await;
        }));
        
        // Create main processing ActorVec that combines BOTH sources
        // Create task handles for proper cleanup
        let data_task = Mutable::new(None);
        let status_task = Mutable::new(None);
        
        let processed_items = ActorVec::new(vec![], clone!((relays, config, data_task, status_task) async move |items_vec| {
            // 🔑 KEY PATTERN: Combine Actor signals and Relay streams with proper task management
            let data_handle = Task::start_droppable(clone!((relays, config, items_vec) async move {
                // Handle data events from Relay stream
                relays.data_events.subscribe().for_each(clone!((items_vec, config, relays) async move |event| {
                    // Get current config state (reactive!)
                    let current_config = config.get();
                    
                    if !current_config.enabled {
                        return; // Skip processing when disabled
                    }
                    
                    match event {
                        DataEvent::ItemReceived { id, data } => {
                            // Process the item according to current config
                            let result = match process_data(&data, &current_config) {
                                Ok(processed) => processed,
                                Err(err) => {
                                    relays.error_occurred.send(format!("Failed to process {}: {}", id, err));
                                    return;
                                }
                            };
                            
                            let processed_item = ProcessedItem {
                                id,
                                result,
                                timestamp: current_timestamp(),
                            };
                            
                            items_vec.lock_mut().push_cloned(processed_item);
                            
                            // Check if we have a full batch
                            let items = items_vec.lock_ref();
                            if items.len() >= current_config.batch_size {
                                let batch: Vec<ProcessedItem> = items.iter().cloned().collect();
                                drop(items); // Release lock
                                
                                // Emit batch results
                                relays.results_ready.send(batch);
                                
                                // Clear processed items
                                items_vec.lock_mut().clear();
                            }
                        },
                        
                        DataEvent::BatchComplete { batch_id } => {
                            // Force emit current batch even if not full
                            let items = items_vec.lock_ref();
                            if !items.is_empty() {
                                let batch: Vec<ProcessedItem> = items.iter().cloned().collect();
                                drop(items);
                                
                                relays.results_ready.send(batch);
                                items_vec.lock_mut().clear();
                            }
                        },
                        
                        DataEvent::ErrorOccurred { error } => {
                            relays.error_occurred.send(error);
                            // Config Actor will handle disabling processing
                        }
                    }
                })).await;
            }));
            
            // Handle status update events from another Relay stream
            let status_handle = Task::start_droppable(clone!((relays) async move {
                relays.status_updates.subscribe().for_each(async |status_msg| {
                    zoon::println!("📊 DataProcessor status: {}", status_msg);
                }).await;
            }));
            
            // Store task handles for cleanup
            data_task.set_neq(Some(data_handle));
            status_task.set_neq(Some(status_handle));
        }));
        
        DataProcessor {
            processed_items,
            config,
            relays,
            data_task,
            status_task,
        }
    }
    
    // Public API for sending events (using RelayBundle)
    pub fn receive_data(&self, id: String, data: Vec<u8>) -> Result<(), RelayError> {
        self.relays.data_events.send(DataEvent::ItemReceived { id, data })
    }
    
    pub fn complete_batch(&self, batch_id: String) -> Result<(), RelayError> {
        self.relays.data_events.send(DataEvent::BatchComplete { batch_id })
    }
    
    pub fn update_status(&self, message: String) -> Result<(), RelayError> {
        self.relays.status_updates.send(message)
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
- **Relay streams**: Data events from `data_relay.subscribe()`
- **Combined logic**: Processing uses current config state + incoming events

**📡 Multi-Stream Actor with Task Management:**
- Multiple `Task::start_droppable()` blocks handling different Relay streams
- TaskHandles stored in Mutables for proper cleanup
- Each stream handler has access to the same Actor state
- Proper async coordination between different event sources

**⚡ Reactive Configuration:**
- Config Actor responds to error events by temporarily disabling processing
- Main processor checks config state reactively during event processing  
- Automatic re-enabling after timeout shows temporal reactive patterns

**🔧 Clean API Separation:**
- Input methods (`receive_data`, `complete_batch`, `update_status`)
- Output subscriptions (`subscribe_to_results`, `subscribe_to_errors`)
- State access (`config_signal`, `items_signal_vec`)

**🔄 RelayBundle Pattern:**
- Group related Relays into a single cloneable struct
- Reduces clone boilerplate from individual relays
- `clone!` macro from enclose crate for clean async closures
- Single clone point instead of multiple per relay

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
        let relay = Relay::<u32>::new();
        let results = Arc::new(Mutex::new(Vec::new()));
        
        let actor = Actor::new(0, async |state| {
            let results = results.clone();
            let relay = relay.clone();
            
            relay.subscribe().for_each(async |value| {
                let results = results.clone();
                let state = state.clone();
                // Simulate processing
                Timer::sleep(10).await;
                state.set(state.get() + value);
                results.lock().unwrap().push(state.get());
            }).await
        });
        
        // Emit events rapidly
        for i in 1..=5 {
            relay.send(i).unwrap();
        }
        
        // Wait for processing
        Timer::sleep(100).await;
        
        // Verify sequential processing
        let results = results.lock().unwrap().clone();
        assert_eq!(results, vec![1, 3, 6, 10, 15]);
    }
    
    #[async_test]
    async fn test_relay_drops_events_without_subscribers() {
        let relay = Relay::<String>::new();
        
        // No subscribers
        assert!(!relay.has_subscribers());
        
        // Should return error when no subscribers
        let result = relay.send("test".to_string());
        assert!(matches!(result, Err(RelayError::NoSubscribers)));
    }
    
    #[async_test]
    async fn test_actor_lifecycle() {
        let actor = Actor::new(42, async |_state| {
            // Actor task
            loop {
                Timer::sleep(1000).await;
            }
        });
        
        assert!(actor.is_running());
        assert_eq!(actor.get(), 42);
        
        // Stop actor
        actor.stop();
        
        // Actor should be stopped
        Timer::sleep(10).await;
        // Note: is_running() would be false after stop
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

### Phase 2: File Management Migration (Week 2)
- [ ] Replace `FileUpdateMessage` with typed relays
- [ ] Convert `TRACKED_FILES` to ActorVec
- [ ] Update all file operation call sites
- [ ] Verify no recursive locks remain

### Phase 3: Critical State Migration (Week 3-4)
- [ ] Convert `SELECTED_VARIABLES` to ActorVec
- [ ] Convert `TIMELINE_CURSOR_POSITION` to Actor
- [ ] Convert search/filter state to Actors
- [ ] Convert config system to type-safe Actor

### Phase 4: UI State Migration (Week 5-6)
- [ ] Convert panel dimensions to Actors
- [ ] Convert dock mode to Actor
- [ ] Convert dialog state to local Actors
- [ ] Update all UI components to use relays

### Phase 5: Cleanup & Optimization (Week 7)
- [ ] Remove all deprecated global mutables
- [ ] Add comprehensive debug logging
- [ ] Performance profiling and optimization
- [ ] Documentation and examples

### Migration Strategy

1. **Incremental Migration**: Start with isolated subsystems
2. **Backward Compatibility**: Keep old APIs temporarily with deprecation warnings
3. **Testing**: Add tests for each migrated component
4. **Monitoring**: Use debug tracing to verify correct behavior

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

## Conclusion

The Actor/Relay architecture solves NovyWave's current state management problems while maintaining Zoon's reactive programming model. By providing clear ownership, type-safe message passing, and controlled mutation points, this architecture will make the codebase more maintainable, debuggable, and reliable.

The migration can be done incrementally, starting with the most problematic areas (file management) and gradually expanding to cover the entire application. Each migrated component becomes more testable and maintainable, providing immediate value even before the full migration is complete.