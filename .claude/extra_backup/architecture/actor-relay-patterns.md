# Actor+Relay Architecture Patterns

Complete reference for NovyWave's Actor+Relay architecture with verified examples and implementation patterns.

## Core Architectural Rules (MANDATORY)

**CRITICAL: NovyWave uses Actor+Relay architecture - NO raw Mutables allowed**

### 1. NO RAW MUTABLES
All state must use Actor+Relay or Atom:

```rust
// ❌ PROHIBITED: Raw global mutables
static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = lazy::default();
static DIALOG_OPEN: Lazy<Mutable<bool>> = lazy::default();

// ❌ PROHIBITED: Functions returning raw Mutables (also violates NO RAW MUTABLES)
pub fn file_tree_cache_mutable() -> zoon::Mutable<HashMap<String, Vec<Item>>> {
    static CACHE: Lazy<Mutable<HashMap<String, Vec<Item>>>> = Lazy::new(|| Mutable::new(HashMap::new()));
    CACHE.clone()  // Still violates NO RAW MUTABLES rule
}

// ✅ REQUIRED: Domain-driven Actors
struct TrackedFiles {
    files: ActorVec<TrackedFile>,
    file_dropped_relay: Relay<Vec<PathBuf>>,
}

// ✅ REQUIRED: Atom for local UI
let dialog_open = Atom::new(false);

// ✅ CORRECT: Use Actor+Relay for shared state
struct FileTreeCache {
    cache: Actor<HashMap<String, Vec<Item>>>,
    cache_updated_relay: Relay<(String, Vec<Item>)>,
}
```

### 2. Event-Source Relay Naming (MANDATORY)
Describe what happened, not what to do:

```rust
// ✅ CORRECT: Describe what happened, not what to do
button_clicked_relay: Relay,              // User clicked button
file_loaded_relay: Relay<PathBuf>,        // File finished loading
input_changed_relay: Relay<String>,       // Input text changed
error_occurred_relay: Relay<String>,      // System error happened

// ❌ PROHIBITED: Command-like naming
add_file: Relay<PathBuf>,                 // Sounds like command
remove_item: Relay<String>,               // Imperative style
set_theme: Relay<Theme>,                  // Action-oriented
```

### 3. Domain-Driven Design (MANDATORY)
Model what it IS, not what it manages:

```rust
// ✅ REQUIRED: Model what it IS, not what it manages
struct TrackedFiles { ... }              // Collection of files
struct WaveformTimeline { ... }          // The timeline itself
struct SelectedVariables { ... }         // Selected variables

// ❌ PROHIBITED: Enterprise abstractions
struct FileManager { ... }               // Artificial "manager"
struct TimelineService { ... }           // Unnecessary "service"
struct DataController { ... }            // Vague "controller"
```

### 4. Cache Current Values Pattern (CRITICAL)
Only inside Actor loops for event response:

```rust
// ✅ ONLY inside Actor loops for event response
let actor = ActorVec::new(vec![], async move |state| {
    let mut cached_username = String::new();  // Cache values
    let mut cached_message = String::new();
    
    loop {
        select! {
            Some(username) = username_stream.next() => cached_username = username,
            Some(message) = message_stream.next() => cached_message = message,
            Some(()) = send_button_stream.next() => {
                // Use cached values when responding to events
                send_message(&cached_username, &cached_message);
            }
        }
    }
});

// ❌ NEVER cache values anywhere else - use signals instead
```

### 5. Relay Data Type Constraints (CRITICAL)
Only send simple cloneable data through relays:

```rust
// ✅ CORRECT: Simple, cloneable data types
file_dropped_relay: Relay<Vec<PathBuf>>,        // Simple data structures
variable_clicked_relay: Relay<String>,          // Primitive types
parse_completed_relay: Relay<(String, Result)>, // Tuples of simple types
initialization_complete_relay: Relay,           // Unit type (empty event)

// ❌ NEVER: Complex types that break Send/Sync bounds
async_operation_relay: Relay<Box<dyn Future>>,  // Futures are not Send
connection_relay: Relay<Arc<Connection>>,       // Complex objects may not be Send
callback_relay: Relay<Box<dyn FnOnce()>>,      // Closures are not Send
```

**Key principle:** Relays are for simple data passing between components. Complex operations should be handled within Actor loops using the "Cache Current Values" pattern, not passed through relays.

## Verified Working Pattern

**✅ VERIFIED IMPLEMENTATION** (from verified chat_example.md):

```rust
use futures::select;

/// Clean Actor+Relay structure with proper separation of concerns
#[derive(Clone)]
struct ChatApp {
    // State managed by Actors - each handles one specific concern
    messages_actor: ActorVec<Message>,
    username_actor: Actor<Username>,
    message_text_actor: Actor<MessageText>,
    
    // Events - event-source based naming with single source per relay
    enter_pressed_relay: Relay,
    send_button_clicked_relay: Relay,
    username_input_changed_relay: Relay<Username>,
    message_input_changed_relay: Relay<MessageText>,
    message_sent_relay: Relay,
}

impl Default for ChatApp {
    fn default() -> Self {
        // Create all relays with streams
        let (enter_pressed_relay, mut enter_pressed_stream) = relay();
        let (send_button_clicked_relay, mut send_button_clicked_stream) = relay();
        let (username_input_changed_relay, mut username_input_changed_stream) = relay();
        let (message_input_changed_relay, mut message_input_changed_stream) = relay();
        let (message_sent_relay, mut message_sent_stream) = relay();
        
        // Simple actors for individual state
        let username_actor = Actor::new(Username::from("DefaultUser"), async move |state| {
            while let Some(name) = username_input_changed_stream.next().await {
                state.set(name);
            }
        });
        
        let message_text_actor = Actor::new(MessageText::default(), async move |state| {
            loop {
                select! {
                    Some(text) = message_input_changed_stream.next() => {
                        state.set(text);
                    }
                    Some(()) = message_sent_stream.next() => {
                        state.set(MessageText::default());  // Clear on send
                    }
                }
            }
        });
        
        // Messages collection with Cache Current Values pattern
        let messages_actor = ActorVec::new(vec![], async move |messages_vec| {
            // ✅ Cache current values as they flow through streams (ONLY in Actor loops)
            let mut cached_username = Username::default();
            let mut cached_message_text = MessageText::default();
            
            let send_trigger_stream = futures::stream::select(
                enter_pressed_stream,
                send_button_clicked_stream
            );
            
            loop {
                select! {
                    // Update cached values when they change
                    Some(username) = username_input_changed_stream.next() => {
                        cached_username = username;
                    }
                    Some(text) = message_input_changed_stream.next() => {
                        cached_message_text = text;
                    }
                    // Use cached values when responding to events
                    Some(()) = send_trigger_stream.next() => {
                        if !cached_message_text.trim().is_empty() {
                            let message = Message { 
                                username: (*cached_username).clone(),
                                text: (*cached_message_text).clone()
                            };
                            messages_vec.lock_mut().push_cloned(message);
                            message_sent_relay.send(()); // Triggers text clear
                        }
                    }
                }
            }
        });
        
        ChatApp {
            messages_actor,
            username_actor,
            message_text_actor,
            enter_pressed_relay,
            send_button_clicked_relay,
            username_input_changed_relay,
            message_input_changed_relay,
            message_sent_relay,
        }
    }
}

// UI integration using signals, not direct state access
impl ChatApp {
    fn username_input(&self) -> impl Element {
        TextInput::new()
            .on_change({
                let relay = self.username_input_changed_relay.clone();
                move |username| { relay.send(Username::from(username)); }
            })
            .text_signal(self.username_actor.signal())  // ✅ Signal-based UI
    }
    
    fn messages_list(&self) -> impl Element {
        Column::new()
            .items_signal_vec(
                self.messages_actor.signal_vec_cloned()
                    .map(|message| render_message(message))  // ✅ Use items_signal_vec
            )
    }
}
```

**Key Patterns Demonstrated:**
1. **Event-source relay naming** - `username_input_changed_relay` not `set_username_relay`
2. **Cache Current Values** - Only inside Actor loops, never in UI
3. **Single concern per Actor** - Each Actor manages one piece of state
4. **Signal-based UI** - UI reads from signals, writes through relays
5. **Clean separation** - Business logic in Actors, UI logic in components

## NovyWave Domain Patterns

### File Management Domain
```rust
// Event-based file operations
struct TrackedFiles {
    files: ActorVec<TrackedFile>,
    file_dropped_relay: Relay<Vec<PathBuf>>,        // Files dropped on UI
    file_selected_relay: Relay<PathBuf>,            // User clicked file
    parse_completed_relay: Relay<(String, ParseResult)>, // Parser finished
}

// Usage: Event emission, not function calls
tracked_files.file_dropped_relay.send(vec![path]);
tracked_files.parse_completed_relay.send((file_id, result));
```

### Variable Selection Domain
```rust
// Variables currently selected for display
struct SelectedVariables {
    variables: ActorVec<SelectedVariable>,
    variable_clicked_relay: Relay<String>,          // User clicked variable
    selection_cleared_relay: Relay,                 // Clear all clicked
    scope_expanded_relay: Relay<String>,            // Scope expanded
}

// Usage: Direct event emission
selected_variables.variable_clicked_relay.send(var_id);
selected_variables.selection_cleared_relay.send(());
```

This eliminates recursive locks while maintaining reactive behavior and complete state traceability.

## Atom for Local UI State

**Replace ALL local Mutables with Atom:**

```rust
// Panel component state
struct PanelState {
    width: Atom<f32>,
    height: Atom<f32>,
    is_collapsed: Atom<bool>,
}

// Dialog component state  
struct DialogState {
    is_open: Atom<bool>,
    filter_text: Atom<String>,
    selected_index: Atom<Option<usize>>,
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            is_open: Atom::new(false),
            filter_text: Atom::new(String::new()),
            selected_index: Atom::new(None),
        }
    }
}
```

## Signal-Based Testing (REQUIRED)

**NO .get() methods - test through signals:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_file_tracking() {
        let tracked_files = TrackedFiles::new();
        
        // Send event through relay
        tracked_files.file_dropped_relay.send(vec![PathBuf::from("test.vcd")]);
        
        // Wait reactively for state change
        let files = tracked_files.files.signal_vec_cloned()
            .to_signal_cloned()
            .to_stream()
            .next().await.unwrap();
            
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, PathBuf::from("test.vcd"));
    }
}
```

**Migration Validation Checklist:**
- [ ] All global Mutables replaced with domain Actors
- [ ] All local Mutables replaced with Atom
- [ ] All relay names follow event-source pattern
- [ ] No Manager/Service/Controller abstractions
- [ ] Event emission replaces direct mutations
- [ ] Signal-based testing throughout

## Signal Handler Patterns

**✅ Correct: Async Signal Handlers**
```rust
// Use for_each with async closure - naturally breaks sync chains
COLLECTION.signal_vec_cloned().for_each(move |data| async move {
    // Runs after current execution completes, locks are dropped
    send_state_message(Message::ProcessData { data });
}).await;
```

**❌ Incorrect: Synchronous Handlers**
```rust
// DON'T: for_each_sync can cause recursive locks
COLLECTION.signal_vec_cloned().for_each_sync(move |data| {
    // Runs immediately while locks may still be held
    send_state_message(Message::ProcessData { data }); // POTENTIAL PANIC!
});
```

## Message Processing Patterns

**✅ Correct: Sequential with Yielding**
```rust
for message in messages {
    Task::next_macro_tick().await;  // ESSENTIAL: Yield to event loop
    process_message(message).await;  // Sequential processing
}
```

**❌ Incorrect: Concurrent Processing**
```rust
for message in messages {
    Task::start(async move {
        process_message(message).await; // All run concurrently - RACES!
    });
}
```

## Debugging State Issues

**Recursive Lock Symptoms:**
```
RuntimeError: unreachable
at std::sys::sync::rwlock::no_threads::RwLock::write
```

**Immediate Actions:**
1. Check for `for_each_sync` handlers that send messages
2. Look for concurrent `Task::start` in message processing loops
3. Verify `Task::next_macro_tick().await` exists between operations
4. Ensure single message processor, not multiple concurrent ones

**Long-term Solutions:**
1. Implement proper Actor Model architecture
2. Use async signal handlers consistently
3. Add event loop yielding to all sequential processing
4. Consider nested Mutables for frequently updated individual items

## CRITICAL Antipatterns

### 1. No Manager/Service/Handler Abstractions

**NEVER create *Manager, *Service, *Controller, or *Handler objects.**

**Why these patterns add complexity through indirection:**
- **DialogManager vs direct AppConfig**: Instead of managing dialog state through an intermediary, connect TreeView directly to AppConfig actors
- **FileManager vs TrackedFiles domain**: Don't create artificial managers - model the actual domain (files are tracked, not "managed")  
- **ServiceLayer vs direct Actor communication**: Services often just forward calls - use Actor+Relay patterns directly

**✅ CORRECT: Objects manage data, not other objects**
```rust
// ✅ GOOD: TrackedFiles manages file data directly
struct TrackedFiles {
    files: ActorVec<TrackedFile>,
    file_dropped_relay: Relay<Vec<PathBuf>>,
}

// ✅ GOOD: AppConfig manages configuration data directly  
struct AppConfig {
    theme_actor: Actor<SharedTheme>,
    file_picker_expanded_directories: Mutable<IndexSet<String>>,
}

// ✅ GOOD: Direct connection - no intermediary
TreeView::new()
    .external_expanded(app_config().file_picker_expanded_directories.clone())
```

**❌ WRONG: Objects that manage other objects through indirection**
```rust
// ❌ BAD: DialogManager doesn't manage data, it manages other components
struct DialogManager {
    file_picker: FilePickerWidget,
    expanded_tracker: ExpandedTracker,  
}

// ❌ BAD: Unnecessary indirection layer
impl DialogManager {
    pub fn expand_directory(&self, path: String) {
        self.expanded_tracker.add_expanded(path);  // Just forwarding!
        self.file_picker.refresh();                 // Complex coupling!
    }
}

// ❌ BAD: Complex routing through abstraction
TreeView::new()
    .external_expanded(dialog_manager().expanded_directories_signal()) // Indirection!
```

**Key principle: Every object should manage concrete data, never other objects. This reduces complexity, eliminates indirection, and makes the code more maintainable.**

### 2. Public Field Architecture (MANDATORY)

**CRITICAL: Use public Relay and Actor fields directly - helper functions are antipatterns**

```rust
// ✅ CORRECT: Direct public field access
struct TrackedFiles {
    pub files: ActorVec<TrackedFile>,                    // Direct access
    pub file_dropped_relay: Relay<Vec<PathBuf>>,        // Public relay field
    pub parse_completed_relay: Relay<(String, ParseResult)>,  // Public relay field
}

// ✅ CORRECT: Direct usage without helper functions
tracked_files.file_dropped_relay.send(vec![path]);
let files_signal = tracked_files.files.signal_vec_cloned();
```

**❌ ANTIPATTERN: Helper functions wrapping field access**
```rust
// ❌ WRONG: Helper functions create unnecessary indirection
impl TrackedFiles {
    pub fn send_file_dropped(&self, files: Vec<PathBuf>) {  // Unnecessary wrapper
        self.file_dropped_relay.send(files);
    }
    
    pub fn get_files_signal(&self) -> impl Signal {         // Unnecessary getter
        self.files.signal_vec_cloned()
    }
}

// ❌ WRONG: Indirect usage through helpers
tracked_files.send_file_dropped(vec![path]);  // Extra indirection
```

**Why helper functions are antipatterns:**
- **Unnecessary complexity** - Direct field access is clearer
- **Cognitive overhead** - Developers must learn both fields AND functions
- **API explosion** - Every field gets multiple wrapper functions
- **Violates Actor+Relay principles** - Relays are designed for direct access

#### Rust Struct Design Philosophy (MANDATORY)

**CRITICAL: Use almost only public fields - treat structs like data, not classes**

Rust structs should be treated as data containers, not object-oriented classes with encapsulation. This prevents access problems and eliminates the need for getter/setter methods.

**✅ CORRECT: Public fields for direct access**
```rust
struct TrackedFiles {
    pub files: ActorVec<TrackedFile>,                    // Public - direct access
    pub files_vec_signal: Mutable<Vec<TrackedFile>>,     // Public - direct access
    pub file_dropped_relay: Relay<Vec<PathBuf>>,         // Public - direct access
}

// ✅ CORRECT: Direct field access
let tracked_files = self.tracked_files.files_vec_signal.get_cloned();
let selected_variables = self.selected_variables.variables_vec_signal.get_cloned();
```

**❌ WRONG: Private fields requiring getter/setter methods**
```rust
struct TrackedFiles {
    files: ActorVec<TrackedFile>,                        // ❌ Private - requires getter
    files_vec_signal: Mutable<Vec<TrackedFile>>,         // ❌ Private - causes errors
}

// ❌ COMPILATION ERROR: private field
let tracked_files = self.tracked_files.files_vec_signal.get_cloned();
//                                     ^^^^^^^^^^^^^^^^ private field

// ❌ ANTIPATTERN: Getter/setter methods
impl TrackedFiles {
    pub fn get_files_vec_signal(&self) -> &Mutable<Vec<TrackedFile>> {  // Unnecessary
        &self.files_vec_signal
    }
}
```

**Key Benefits of Public Fields:**
- **Direct access** - No compilation errors from private field access
- **No getter/setter boilerplate** - Eliminates unnecessary methods
- **Data-oriented design** - Structs are data containers, not objects
- **Rust idioms** - Follows Rust's preference for data over encapsulation
- **Simplicity** - Less cognitive overhead than OOP encapsulation

**When to use private fields (rare exceptions):**
- Internal implementation details that would break if exposed
- Fields requiring invariant maintenance through methods
- Complex state that needs validation during updates

**General rule: Default to `pub` fields unless there's a specific reason for privacy.**

### 3. zoon::Task Prohibition (CRITICAL)

**CRITICAL: NEVER use zoon::Task - use Actors instead for all event handling**

```rust
// ❌ ANTIPATTERN: zoon::Task for event handling
zoon::Task::start(async move {
    some_signal.for_each(|value| {
        process_value(value);
        async {}
    }).await;
});

// ❌ ANTIPATTERN: zoon::Task for bi-directional sync
zoon::Task::start(async move {
    actor_signal.for_each(|data| {
        mutable.set_neq(data);
        async {}
    }).await;
});
```

**✅ CORRECT: Use Actors for all event processing**
```rust
// ✅ CORRECT: Actor handles event processing
let processing_actor = Actor::new(State::default(), async move |state| {
    loop {
        select! {
            Some(value) = value_stream.next() => {
                // Process in proper Actor context
                process_value_with_state(value, &mut state);
            }
        }
    }
});

// ✅ CORRECT: Actors handle bi-directional sync
struct SyncActor {
    mutable: Mutable<Data>,
    actor: Actor<Data>,
}
```

**Key Rule: If you're using zoon::Task for anything other than one-off operations (like debounced saves), use Actor instead.**

#### Internal Relay Pattern for Eliminating zoon::Task

**VERIFIED SOLUTION: When zoon::Task is used for async operations within Actor contexts, use internal relays instead**

This pattern completely eliminates zoon::Task while maintaining proper async handling within Actor select! loops:

```rust
// ✅ CORRECT: Add internal relay to Actor struct
#[derive(Clone)]
pub struct TrackedFiles {
    pub files: ActorVec<TrackedFile>,
    pub file_parse_requested_relay: Relay<String>, // Internal relay for async operations
    // ... other fields
}

impl TrackedFiles {
    pub async fn new() -> Self {
        let (file_parse_requested_relay, mut file_parse_requested_stream) = relay::<String>();
        
        let files = ActorVec::new(vec![], async move |files_handle| {
            loop {
                select! {
                    // ... other event streams
                    
                    // ✅ Handle async operations within Actor context
                    parse_requested = file_parse_requested_stream.next() => {
                        if let Some(file_path) = parse_requested {
                            // ✅ CORRECT: Direct async call within Actor - NO zoon::Task!
                            send_parse_request_to_backend(file_path).await;
                        }
                    }
                }
            }
        });
        
        Self { files, file_parse_requested_relay }
    }
}

// ✅ Usage: Direct relay emission instead of zoon::Task
fn trigger_async_operation(file_path: String) {
    // Instead of: zoon::Task::start(async_operation(file_path))
    tracked_files.file_parse_requested_relay.send(file_path); // ✅ Pure relay event
}
```

**Benefits of Internal Relay Pattern:**
- ✅ **Complete zoon::Task elimination** - No Task usage anywhere
- ✅ **Proper async handling** - await works correctly in Actor select! loops  
- ✅ **Actor+Relay compliance** - Maintains architectural principles
- ✅ **Internal organization** - Async operations handled within domain boundaries
- ✅ **Clean event flow** - Direct relay emission → Actor processing → async execution

**When to Use This Pattern:**
- Replacing zoon::Task::start(async_function()) calls within Actor contexts
- Backend communication from within Actors (file parsing, network requests)
- Any async operation that was previously wrapped in zoon::Task for event handling
- Operations that need to be triggered from within Actor select! loops

### 4. Data Bundling Struct Antipattern (CRITICAL)

**ANTIPATTERN: Artificial struct groupings that force unrelated data to update together**

```rust
// ❌ WRONG: Data bundling struct - artificial grouping of unrelated dimensions
#[derive(Clone, Debug)]
struct PanelDimensions {
    pub files_panel_width: f32,           // Files panel concern
    pub timeline_panel_height: f32,       // Timeline panel concern  
    pub variables_name_column_width: f32, // Variables table concern
    pub variables_value_column_width: f32, // Variables table concern
}

// ❌ PROBLEMS: Forces all dimensions to update together
Actor<PanelDimensions>  // Single change triggers entire struct update
Relay<PanelDimensions>  // Bundles unrelated dimension changes
```

**Why this is harmful:**
- **Artificial coupling** - Groups unrelated concerns (files + timeline + variables)
- **Update overhead** - Single dimension change forces entire struct recreation
- **Signal noise** - Unrelated components get notified of irrelevant changes  
- **Construction complexity** - Must build entire struct for single field updates
- **Testing difficulty** - Cannot test individual dimension concerns in isolation

**✅ CORRECT: Individual actors for each concern**
```rust
// ✅ GOOD: Separate actors for independent concerns
struct PanelConfig {
    pub files_panel_width_actor: Actor<f32>,          // Files panel only
    pub files_panel_height_actor: Actor<f32>,         // Files panel only
    pub timeline_panel_height_actor: Actor<f32>,      // Timeline only
    pub variables_name_column_width_actor: Actor<f32>, // Variables only
    pub variables_value_column_width_actor: Actor<f32>, // Variables only
    
    // Individual relays for precise change notifications
    pub files_width_changed_relay: Relay<f32>,
    pub timeline_height_changed_relay: Relay<f32>,
    pub name_column_width_changed_relay: Relay<f32>,
}
```

**Benefits of separated actors:**
- **Precise updates** - Only affected components get notifications
- **Independent testing** - Test each dimension concern separately
- **Clear ownership** - Each actor has single responsibility
- **Performance** - No unnecessary struct construction/destruction
- **Reactive precision** - UI updates only when relevant data changes

**When bundling IS appropriate:**
- **Cohesive data** - Fields that logically belong together (e.g., `Point { x, y }`)
- **Atomic updates** - When all fields must change together for consistency
- **Domain modeling** - When struct represents a real-world concept

**Key Rule: Avoid bundling data just for "organization" - use proper domain modeling instead.**

### 5. Atom for Local UI State

> See "Atom for Local UI State" section above for examples.

**Use Atom for:** Dialog visibility, hover effects, search filters, animation states, form inputs (before submission).

### 6. NO Temporary Code Rule

**CRITICAL: Never create temporary solutions or bridge code**

- **NO "temporary" signal updates** - Either implement proper Actor+Relay or use existing working patterns
- **NO TODO comments** for "will implement later" - Do it right the first time or use established patterns
- **Use Atoms for simple UI logic** - Hovering, focus states, local toggles, UI-only state
- **Use Actor+Relay for domain logic** - Business state, cross-component coordination, persistent data

**✅ CORRECT: Atom for simple UI states**
```rust
// Hover effects, focus states, UI toggles - use Atom directly
let button_hovered = Atom::new(false);
let panel_collapsed = Atom::new(false);
let input_focused = Atom::new(false);

// UI event handlers
.on_hovered_change(move |is_hovered| button_hovered.set_neq(is_hovered))
.s(Background::new().color_signal(button_hovered.signal().map(|hovered| {
    if *hovered { hover_color() } else { normal_color() }
})))
```

**❌ WRONG: Creating temporary bridge code**
```rust
// Don't create "temporary" solutions that bypass proper architecture
pub fn open_file_dialog() {
    domain.dialog_opened_relay.send(());
    
    // ❌ TEMPORARY: Also update signals directly until Actor processors are implemented
    if let Some(signals) = SIGNALS.get() {
        signals.dialog_visible_mutable.set_neq(true);  // Bridge code!
    }
}
```

#### ANTIPATTERN: "Fix Compilation First, Implement Later"

**❌ CRITICAL ANTIPATTERN: Compilation-driven development**
```rust
// WRONG: Placeholder just to make it compile
pub fn get_variables_from_tracked_files(scope_id: &str) -> Vec<VariableWithContext> {
    Vec::new()  // TODO: Implement later
}
```

**Why this is catastrophically harmful:**
- **Technical debt accumulates** - "Later" often never comes
- **Breaks functionality** - Code compiles but doesn't work
- **False sense of progress** - Green build status masks broken features
- **Context switching cost** - Returning to implement requires re-understanding the problem
- **Hides architectural issues** - Compilation success doesn't mean correct design

**✅ CORRECT: Implement properly or don't implement at all**
```rust
// Either implement the real functionality:
pub fn get_variables_from_tracked_files(scope_id: &str) -> Vec<VariableWithContext> {
    let tracked_files = crate::state::tracked_files();
    let files = tracked_files.files_vec_signal.get_cloned();
    // ... proper implementation using existing data structures
}

// Or comment out the call site until ready:
// variables_display_signal(tracked_files.clone(), selected_variables.clone())
```

**Key Principle:** Working compilation with broken functionality is worse than compilation errors with correct architecture.

## Advanced Implementation Patterns

> See "Core Architectural Rules" section above for Event-Source Relay Naming and Cache Current Values patterns.

### CRITICAL ANTIPATTERN: State Access Outside Actor Loops

**❌ NEVER DO: get() + set() pattern**
```rust
// ANTIPATTERN 1: Race condition prone - value can change between get() and set()
let current_theme = state.get();  // Read
let new_theme = toggle_theme(current_theme);  // Compute
state.set(new_theme);  // Write - but state may have changed!

// ANTIPATTERN 2: Unnecessary cloning for every access
fn toggle_theme() {
    let theme = THEME_STATE.get();  // Always clones entire value
    let new_theme = match theme {
        Theme::Light => Theme::Dark,
        Theme::Dark => Theme::Light,
    };
    THEME_STATE.set(new_theme);  // Another clone + signal emission
}
```

**❌ EVEN WORSE: Internal caching to "optimize" get/set**
```rust
// ANTIPATTERN 3: Shadowing actor state with manual cache
let mut cached_theme = Theme::Light;  // Manual cache outside Actor
Task::start(THEME_STATE.signal().for_each_sync(move |theme| {
    cached_theme = theme;  // Trying to "optimize" by caching
}));

// Now you have TWO sources of truth - recipe for bugs
```

**✅ CORRECT: Direct lock_mut() manipulation**
```rust
// ✅ ATOMIC: Single lock covers read+modify+write operation
{
    let mut theme = state.lock_mut();
    let old_theme = *theme;  // Read current value
    *theme = match *theme {  // Atomic modify
        Theme::Light => Theme::Dark,
        Theme::Dark => Theme::Light,
    };
    // Lock automatically dropped, signals fire atomically
}
```

**Why lock_mut() is correct:**
- **Atomic operations** - No race conditions between read and write
- **No cloning** - Direct mutation of the actual state
- **Single source of truth** - No shadow caches to get out of sync  
- **Automatic signaling** - Changes immediately trigger reactive updates
- **Lock scope control** - Explicit lifetime management with block scope

**When this pattern applies:**
- Toggle operations (theme, dock mode, checkboxes)
- Increment/decrement counters  
- State machines with transitions
- Any read-modify-write sequence that should be atomic

## Context Object Pattern for Utility Functions (MANDATORY)

**CRITICAL USER GUIDANCE: "I wanted you to group as much utility and UI functions into multiple objects or big objects and then the functions would just take domain objects or whatever from `self` when needed - think about `self` as a passing Ctx or similar constructs"**

### Core Principle: Self as Domain Context

Instead of cascading domain parameters through individual functions, create context objects that encapsulate domain access. This eliminates parameter cascading and creates clean dependency injection patterns.

**✅ CORRECT: Context Object Pattern**
```rust
/// Context object holding domain references
struct TimelineContext {
    pub tracked_files: TrackedFiles,
    pub selected_variables: SelectedVariables,
    pub waveform_timeline: WaveformTimeline,
}

impl TimelineContext {
    pub fn new(
        tracked_files: TrackedFiles,
        selected_variables: SelectedVariables,
        waveform_timeline: WaveformTimeline,
    ) -> Self {
        Self { tracked_files, selected_variables, waveform_timeline }
    }
    
    /// Domain access through self - no parameter cascading
    pub fn get_maximum_timeline_range(&self) -> Option<(f64, f64)> {
        let tracked_files = self.tracked_files.files_vec_signal.get_cloned();
        let selected_file_paths = self.get_selected_variable_file_paths();
        // All domain access through self.domain_name
    }
    
    pub fn get_selected_variable_file_paths(&self) -> HashSet<String> {
        let selected_vars = self.selected_variables.variables_vec_signal.get_cloned();
        selected_vars.iter().filter_map(|var| var.file_path()).collect()
    }
}

/// Usage: Create context once, use methods everywhere
let timeline_ctx = TimelineContext::new(tracked_files, selected_variables, waveform_timeline);
let range = timeline_ctx.get_maximum_timeline_range();
let paths = timeline_ctx.get_selected_variable_file_paths();
```

**❌ WRONG: Parameter Cascading Antipattern**
```rust
// ❌ ANTIPATTERN: Cascading parameters everywhere
pub fn get_maximum_timeline_range(
    tracked_files: &TrackedFiles,
    selected_variables: &SelectedVariables,
) -> Option<(f64, f64)> {
    let selected_file_paths = get_selected_variable_file_paths(selected_variables);
    // More parameter passing...
}

pub fn get_selected_variable_file_paths(
    selected_variables: &SelectedVariables  // Parameter cascade continues
) -> HashSet<String> {
    // Function needs to be updated every time signature changes
}
```

### Benefits of Context Object Pattern

1. **Eliminates Parameter Cascading**: No more updating 20+ function signatures when adding a domain
2. **Clean Dependency Injection**: Domains injected once at context creation
3. **Method Grouping**: Related utility functions naturally grouped together
4. **Self as Context**: `self` provides domain access without explicit parameters
5. **Single Responsibility**: Each context object handles one functional area

### Implementation Strategy

1. **Group Related Functions**: Identify utility functions that work together
2. **Create Context Struct**: Define struct with needed domain references
3. **Convert Functions to Methods**: Transform `fn(domain_params)` → `fn(&self)`
4. **Update Call Sites**: Replace function calls with context method calls
5. **Domain Access via Self**: Use `self.domain_name` instead of parameters

### Context Object Examples for NovyWave

```rust
// Timeline utilities
struct TimelineContext {
    tracked_files: TrackedFiles,
    selected_variables: SelectedVariables,
    waveform_timeline: WaveformTimeline,
}

// UI rendering utilities  
struct UIContext {
    tracked_files: TrackedFiles,
    selected_variables: SelectedVariables,
    app_config: AppConfig,
}

// Connection and messaging utilities
struct ConnectionContext {
    tracked_files: TrackedFiles,
    selected_variables: SelectedVariables,
    waveform_timeline: WaveformTimeline,
}
```

**Key Rule: Think of `self` as a passing context (Ctx) that provides domain access without explicit parameter passing.**

## Standalone Derived State Actors

### Centralize Cross-Domain Computed Values

**CRITICAL: For derived data needed by multiple domains, create standalone actors instead of duplicating computation logic.**

**Problem Pattern - Scattered Computation:**
```rust
// ❌ WRONG: Same computation duplicated across different actors
// In WaveformTimeline actor:
if let Some((min_time, max_time)) = compute_timeline_range_from_files() { ... }

// In ZoomController actor:
if let Some((min_time, max_time)) = compute_timeline_range_from_files() { ... }

// In ResetZoom handler:
if let Some((min_time, max_time)) = compute_timeline_range_from_files() { ... }
```

**✅ CORRECT: Standalone Derived State Actor**
```rust
/// Dedicated actor for timeline range - single source of truth
#[derive(Clone, Debug)]
pub struct MaximumTimelineRange {
    pub range: Actor<Option<(f64, f64)>>,
    pub range_updated_relay: Relay<Option<(f64, f64)>>,
}

impl MaximumTimelineRange {
    pub async fn new(
        tracked_files: TrackedFiles,
        selected_variables: SelectedVariables,
    ) -> Self {
        let (range_updated_relay, mut range_updated_stream) = relay();
        
        let range = Actor::new(None, async move |state| {
            loop {
                select! {
                    Some(new_range) = range_updated_stream.next() => {
                        state.set(new_range);
                    }
                }
            }
        });
        
        // Background computation - updates when source data changes
        let timeline_context = TimelineContext { tracked_files, selected_variables };
        let range_relay = range_updated_relay.clone();
        zoon::Task::start(async move {
            tracked_files.files_signal().for_each_sync(move |_files| {
                let new_range = timeline_context.get_maximum_timeline_range();
                range_relay.send(new_range);
            });
        });
        
        Self { range, range_updated_relay }
    }
    
    pub fn range_signal(&self) -> impl Signal<Item = Option<(f64, f64)>> {
        self.range.signal()
    }
}
```

**Using Cached Values in Other Actors:**
```rust
// Other actors can cache the derived value
let maximum_timeline_range = MaximumTimelineRange::new(tracked_files, selected_variables).await;

let zoom_actor = Actor::new(ZoomState::default(), {
    let range_actor = maximum_timeline_range.clone();
    async move |state| {
        // Cache current values pattern
        let mut cached_timeline_range: Option<(f64, f64)> = None;
        let mut range_stream = range_actor.range_signal().to_stream();
        
        loop {
            select! {
                // Update cached value when range changes
                range = range_stream.next() => {
                    if let Some(new_range) = range {
                        cached_timeline_range = new_range;
                    }
                }
                // Use cached value in business logic
                reset_event = reset_stream.next() => {
                    if let Some((min_time, max_time)) = cached_timeline_range {
                        // Use range for zoom calculations
                    }
                }
            }
        }
    }
});
```

**Key Benefits:**
- ✅ **Single Source of Truth** - Timeline range computed once, used everywhere
- ✅ **No State Scattering** - Derived data centralized in dedicated actor
- ✅ **Automatic Updates** - All consumers get updates when source data changes
- ✅ **Clean Architecture** - No cross-domain dependencies in business logic actors
- ✅ **Performance** - Computation happens once, not per consumer

**When to Use:**
- Derived data needed by multiple domains/actors
- Complex computations that shouldn't be duplicated
- Cross-domain state that doesn't belong to any single domain
- Values that update when source data changes

**Pattern Rule:** Centralize derived state instead of scattering computation logic across the codebase.

## State Management Migration Patterns

### Bool → Unit Event Transition
For one-time lifecycle events, use `Relay` (unit) instead of `Relay<bool>`:
```rust
// ❌ let (init_relay, _) = relay::<bool>(); init_relay.send(true);
// ✅ let (init_relay, _) = relay(); init_relay.send(()); // Event = signal
```

### Atom → Actor Encapsulation
Public Atoms allow external `.set()` calls that bypass domain logic. Use Actor for encapsulation:
```rust
// ❌ pub status: Atom<bool> // External code can corrupt state
// ✅ pub status_actor: Actor<bool> // Read-only signal access only
```

### Complex Type Elimination
If you have `Atom<Option<Rc<RefCell<T>>>>`, use `Actor<()>` with inline state variables instead:
```rust
// ❌ Multiple indirection layers fighting ownership system
// ✅ Actor::new((), async move |_| { let mut renderer = None; loop { ... } })
```

**Recognition patterns:** Multiple angle brackets, defensive `try_borrow_mut()`, manual Clone implementations.