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

// ✅ REQUIRED: Domain-driven Actors
struct TrackedFiles {
    files: ActorVec<TrackedFile>,
    file_dropped_relay: Relay<Vec<PathBuf>>,
}

// ✅ REQUIRED: Atom for local UI
let dialog_open = Atom::new(false);
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

### 4. Atom for Local UI State (MANDATORY)

**CRITICAL: Use Atom for local view state, Actor+Relay for domain state**

```rust
// ✅ CORRECT: Atom for local UI concerns
struct DialogState {
    pub is_open: Atom<bool>,              // Local UI state
    pub filter_text: Atom<String>,        // Local UI state
    pub hover_index: Atom<Option<usize>>, // Local UI state
}

// ✅ CORRECT: Actor+Relay for domain concerns
struct TrackedFiles {
    pub files: ActorVec<TrackedFile>,     // Domain data
    pub file_dropped_relay: Relay<Vec<PathBuf>>,  // Domain events
}

// ✅ CORRECT: Usage pattern
let dialog_state = DialogState::default();
dialog_state.is_open.set(true);  // Direct Atom usage

tracked_files.file_dropped_relay.send(files);  // Direct Relay usage
```

**Local UI State Examples:**
- Dialog visibility, panel collapse state
- Hover effects, focus states  
- Search filters, sort order
- Animation states, loading spinners
- Form input values (before submission)

### 5. NO Temporary Code Rule

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

## Advanced Implementation Patterns

### Event-Source Relay Naming (MANDATORY)

**CRITICAL RULE: Relay names must identify the source of events, not the destination or purpose**

**Pattern:** `{specific_source}_{event}_relay`

**✅ CORRECT - Source identification:**
```rust
// User interactions - identify WHO initiated the event
theme_button_clicked_relay: Relay,           // Theme button clicked
dock_button_clicked_relay: Relay,            // Dock mode button clicked  
file_drop_area_dropped_relay: Relay<Vec<PathBuf>>, // Files dropped on drop area
search_input_changed_relay: Relay<String>,   // Search input text changed
variable_tree_item_clicked_relay: Relay<String>, // Tree item clicked

// System events - identify WHAT system component triggered
file_parser_completed_relay: Relay<ParseResult>, // File parser finished
network_error_occurred_relay: Relay<String>,     // Network component errored
config_loader_finished_relay: Relay<Config>,     // Config loader completed

// UI component events - identify WHICH component
main_dialog_opened_relay: Relay,             // Main dialog opened
timeline_panel_resized_relay: Relay<(f32, f32)>, // Timeline panel resized
```

**❌ PROHIBITED - Vague or command-like naming:**
```rust
// ❌ Generic "request" - doesn't identify source
theme_toggle_requested_relay: Relay,         // WHO requested? Button? Keyboard? API?
file_add_requested_relay: Relay<PathBuf>,    // WHERE did the request come from?

// ❌ Command-like naming
add_file: Relay<PathBuf>,                    // Sounds like imperative command
remove_item: Relay<String>,                  // Action-oriented
set_theme: Relay<Theme>,                     // Imperative verb
update_config: Relay<Config>,                // Generic command
```

**Key Principle: Single Source, Multiple Subscribers**
- **One source location** can send events through a relay (e.g., only theme button sends theme_button_clicked events)
- **Multiple subscribers** can listen to the same relay (UI updates, config saving, logging, etc.)
- **Source identification** in the name makes debugging and code navigation easy

### Cache Current Values Pattern (DETAILED)

**⚠️ MANDATORY: This is the ONLY acceptable place to cache values in Actor+Relay architecture**

The "Cache Current Values" pattern is used **EXCLUSIVELY inside Actor processing loops** to maintain current state values for use when responding to events.

```rust
// ✅ CORRECT: Cache values ONLY inside Actor loops for event response
let actor = ActorVec::new(vec![], async move |state| {
    // Cache current values as they flow through streams
    let mut current_filter = String::new();
    let mut selected_items = Vec::new();
    
    loop {
        select! {
            // Update cached values when they change
            Some(filter) = filter_stream.next() => {
                current_filter = filter;
            }
            Some(items) = selection_stream.next() => {
                selected_items = items;
            }
            // Use cached values when responding to events
            Some(()) = action_button_stream.next() => {
                process_selection(&current_filter, &selected_items);
            }
        }
    }
});
```

**Key Rules:**
- **ONLY cache inside Actor loops** - never in UI components or globally
- **Use caching for event response** - when you need multiple values at once
- **Otherwise use signals** - for all other state access
- **Never use raw Mutables for caching** - defeats the architecture

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

This comprehensive guide provides everything needed to implement proper Actor+Relay architecture in NovyWave.