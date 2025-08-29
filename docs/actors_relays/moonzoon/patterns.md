# Actor+Relay Patterns & Migration Guide

A comprehensive guide to modern Actor+Relay patterns and migration strategies for NovyWave, extracted from practical implementation experience.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Critical Pattern: Cache Current Values](#critical-pattern-cache-current-values)
3. [Modern Actor+Relay Patterns](#modern-actorrelay-patterns)
4. [Migration Patterns](#migration-patterns)
5. [Atom Helper](#atom-helper)
6. [Performance Best Practices](#performance-best-practices)
7. [Event-Driven Architecture Patterns](#event-driven-architecture-patterns)

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

### Core API Pattern

> **📖 Complete API Reference:** See [api.md](api.md) for the full API specification.

```rust
/// Creates a new Relay with an associated stream, following Rust's channel pattern.
/// This is the idiomatic way to create a Relay for use with Actors.
pub fn relay<T>() -> (Relay<T>, impl Stream<Item = T>) {
    let relay = Relay::new();
    let stream = relay.subscribe();
    (relay, stream)
}

// Usage example:
let (increment, mut increment_stream) = relay();
let (decrement, mut decrement_stream) = relay();

let counter = Actor::new(0, async move |state| {
    loop {
        select! {
            Some(()) = increment_stream.next() => {
                state.update(|n| n + 1);
            }
            Some(()) = decrement_stream.next() => {
                state.update(|n| n.saturating_sub(1));
            }
        }
    }
});
```

## Critical Pattern: Cache Current Values

### ⚠️ The ONLY Acceptable Place for Value Caching

The "Cache Current Values" pattern is a **CRITICAL** pattern that must be understood correctly. It is used **EXCLUSIVELY inside Actor processing loops** to maintain current state values for use when responding to events.

### The Pattern Explained

```rust
// ✅ CORRECT: Cache values ONLY inside Actor loops for event response
let chat_actor = ActorVec::new(vec![], async move |messages| {
    // Cache current values as they flow through streams
    let mut current_username = String::new();
    let mut current_message = String::new();
    
    loop {
        select! {
            // Update cached value when username changes
            Some(username) = username_input_stream.next() => {
                current_username = username;
            }
            
            // Update cached value when message changes
            Some(message) = message_input_stream.next() => {
                current_message = message;
            }
            
            // Use cached values when responding to send event
            Some(()) = send_button_stream.next() => {
                if !current_message.trim().is_empty() {
                    let chat_message = ChatMessage {
                        username: current_username.clone(),
                        text: current_message.clone(),
                    };
                    connection.send(chat_message);
                    
                    // Clear message after sending
                    current_message.clear();
                    message_input_cleared_relay.send(());
                }
            }
        }
    }
});
```

### Why This Pattern Exists

1. **Synchronous Access Requirement**: When an event occurs (like button click), you often need multiple values immediately
2. **Signals Are Async**: You can't query signals synchronously inside the select! loop
3. **Maintains Architecture**: Caching inside the Actor maintains single-point mutation principle
4. **Practical Necessity**: Without this pattern, complex event handling would be impossible

### Rules for Using This Pattern

#### ✅ DO:
- Cache values **ONLY inside Actor processing loops**
- Update cached values when their streams emit new values
- Use cached values when responding to action events
- Clear/reset cached values after use when appropriate

#### ❌ DON'T:
- Cache values in UI components
- Use global variables for caching
- Create Mutables for caching outside Actors
- Try to .get() from Actors (no such method exists)

### Real-World Examples

#### Form Submission
```rust
let form_actor = Actor::new(FormState::default(), async move |state| {
    // Cache all form fields
    let mut name = String::new();
    let mut email = String::new();
    let mut phone = String::new();
    let mut message = String::new();
    
    loop {
        select! {
            Some(v) = name_input_stream.next() => name = v,
            Some(v) = email_input_stream.next() => email = v,
            Some(v) = phone_input_stream.next() => phone = v,
            Some(v) = message_input_stream.next() => message = v,
            
            Some(()) = submit_button_stream.next() => {
                // Use all cached values for submission
                let form = FormData { name, email, phone, message };
                if validate_form(&form) {
                    submit_to_backend(form).await;
                    form_submitted_relay.send(());
                }
            }
        }
    }
});
```

#### File Dialog with Filters
```rust
let dialog_actor = Actor::new(DialogState::default(), async move |state| {
    // Cache dialog state
    let mut search_filter = String::new();
    let mut selected_files = Vec::new();
    let mut current_directory = PathBuf::from("/");
    
    loop {
        select! {
            Some(filter) = search_input_stream.next() => {
                search_filter = filter;
                update_filtered_view(&search_filter);
            }
            
            Some(files) = file_selection_stream.next() => {
                selected_files = files;
            }
            
            Some(dir) = directory_changed_stream.next() => {
                current_directory = dir;
                selected_files.clear();
            }
            
            Some(()) = open_button_stream.next() => {
                // Use cached selections
                for file in &selected_files {
                    let full_path = current_directory.join(file);
                    open_file(full_path).await;
                }
                dialog_closed_relay.send(());
            }
        }
    }
});
```

#### Timeline Cursor with Multiple Values
```rust
let timeline_actor = Actor::new(TimelineState::default(), async move |state| {
    // Cache timeline state
    let mut cursor_position = 0.0;
    let mut zoom_level = 1.0;
    let mut visible_range = (0.0, 100.0);
    
    loop {
        select! {
            Some(pos) = cursor_moved_stream.next() => {
                cursor_position = pos;
                state.set_cursor(pos);
            }
            
            Some(zoom) = zoom_changed_stream.next() => {
                zoom_level = zoom;
                // Recalculate visible range based on cursor and zoom
                visible_range = calculate_range(cursor_position, zoom_level);
                state.set_range(visible_range);
            }
            
            Some(()) = center_cursor_stream.next() => {
                // Use cached values to center view
                let center = (visible_range.0 + visible_range.1) / 2.0;
                cursor_position = center;
                state.set_cursor(center);
            }
        }
    }
});
```

### Common Mistakes to Avoid

```rust
// ❌ WRONG: Global caching
static mut CACHED_VALUE: String = String::new();  // NEVER!

// ❌ WRONG: Caching in UI components
fn my_component() -> impl Element {
    let mut cached = String::new();  // NO! Use signals instead
    Button::new().on_click(move || {
        send_value(&cached);  // Wrong place for caching
    })
}

// ❌ WRONG: Using Mutables for caching
let cached_username = Mutable::new(String::new());  // Defeats architecture!

// ❌ WRONG: Trying to get values synchronously from Actors
let current = some_actor.get();  // No .get() method exists!
```

### Summary

The "Cache Current Values" pattern is:
- **Essential** for practical Actor+Relay usage
- **Restricted** to Actor processing loops only
- **Never** used anywhere else in the codebase
- **The bridge** between async streams and synchronous event handling

This pattern maintains architectural purity while enabling real-world functionality. When in doubt, ask: "Am I inside an Actor's async processing loop?" If not, use signals instead.

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
// Clean and simple with relay()
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

### Multi-Stream Actor Pattern with join!()

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
                        zoon::println!("🔧 Config updated: {:?}", config);
                        // Update processing behavior based on config
                    }
                },
                
                // Stream 3: Timer events
                async {
                    while let Some(tick) = timer_stream.next().await {
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

### Advanced Multi-Stream Pattern with select!()

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
│   │   ├── Complex coordination? → Consider separate coordinating Actor
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

### Discrete Event Pattern (Unit Relays)

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
                        perform_save().await;
                        state.set_neq(AppMode::Normal);
                    }
                    Some(()) = load_stream.next() => {
                        state.set_neq(AppMode::Loading);
                        perform_load().await;
                        state.set_neq(AppMode::Normal);
                    }
                    Some(()) = exit_stream.next() => {
                        state.set_neq(AppMode::Exiting);
                        cleanup().await;
                    }
                    Some(()) = help_stream.next() => {
                        state.set_neq(AppMode::ShowingHelp);
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

### Complete Architecture Example

**Modern File Manager with All Patterns:**
```rust
/// Complete example showing all modern patterns
#[derive(Clone)]
struct ModernFileManager {
    // Core state managed by Actor
    files: ActorVec<TrackedFile>,
    
    // Events using relay() pattern
    pub add_file: Relay<PathBuf>,
    pub remove_file: Relay<String>,
    pub file_selected: Relay<String>,
    
    // Local UI state using Atom
    pub filter_text: Atom<String>,
    pub is_loading: Atom<bool>,
    pub selected_count: Atom<usize>,
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
            // Atom for all local UI state
            filter_text: Atom::new(String::new()),
            is_loading: Atom::new(false),
            selected_count: Atom::new(0),
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

## Atom Helper

### ONLY EXCEPTION: Atom Helper
The `Atom` helper is acceptable for truly local UI state (button hover, dropdown open/closed) as it's still a controlled abstraction:

```rust
// ACCEPTABLE: Atom helper for local UI only
let is_hovered = Atom::new(false);
```

### Complete Atom Implementation (Canonical Version)

```rust
/// Unified helper for local UI state - uses Actor+Relay architecture internally
#[derive(Clone, Debug)]
pub struct Atom<T: Clone + Send + Sync + 'static> {
    pub value: Actor<T>,
    pub setter: Relay<T>,
}

impl<T: Clone + Send + Sync + 'static> Atom<T> {
    fn new(initial: T) -> Self {
        let (setter, mut setter_stream) = relay();
        
        let value = Actor::new(initial, async move |state| {
            while let Some(new_value) = setter_stream.next().await {
                state.set_neq(new_value);
            }
        });
        
        Atom { value, setter }
    }
    
    // Convenient methods that delegate to Actor+Relay
    fn set(&self, value: T) { self.setter.send(value); }
    fn signal(&self) -> impl Signal<Item = T> { self.value.signal() }
}

// Usage pattern: Replace all global Mutables with local Atom
struct DialogState {
    is_dialog_open: Atom<bool>,
    filter_text: Atom<String>,
    selected_index: Atom<Option<usize>>,
    hover_state: Atom<bool>,
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            is_dialog_open: Atom::new(false),
            filter_text: Atom::new(String::new()),
            selected_index: Atom::new(None),
            hover_state: Atom::new(false),
        }
    }
}
```

**Important Rules:**
- **NEVER use raw `Mutable<T>` directly in Actor+Relay architecture!**
- The entire purpose of this architecture is to eliminate uncontrolled state mutations
- **ALL state must be managed through Actors**
- **True Actor+Relay Architecture**: Uses Actor+Relay internally, no raw Mutable violations

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
    values: ActorMap<String, i32>,  // String lookup overhead
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

## Event-Driven Architecture Patterns

### Performance Benefits

**Measured Improvements from Pattern Adoption:**

1. **Reduced Boilerplate**: ~60% less code using `relay()` vs clone! macros
2. **Better Debugging**: Imperative while loops easier to step through than nested async closures  
3. **Cleaner Error Handling**: Direct relay access eliminates Result<(), RelayError> propagation
4. **Unified State Management**: Atom eliminates inconsistent Mutable usage patterns
5. **Concurrent Processing**: join!() pattern enables true multi-stream concurrency

### Migration Strategy

**Step-by-Step Modernization:**

1. **Replace clone! patterns** with `relay()`
2. **Convert .for_each() to while loops** for easier debugging
3. **Introduce Atom** for all local UI state (dialog open/closed, filter text, etc.)
4. **Use join!() for multi-stream** scenarios instead of multiple Task::start calls
5. **Eliminate raw Mutable usage** in favor of either Actor (shared state) or Atom (local state)

This approach provides the cleanest path forward for new Actor+Relay implementations and systematic modernization of existing code.

---

These patterns, observed consistently across the examples, provide the foundation for high-performance Actor+Relay applications and represent the evolution from complex global state management to clean, event-driven architecture.