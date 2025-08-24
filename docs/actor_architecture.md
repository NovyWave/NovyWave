# Actor Architecture for NovyWave

## Executive Summary

NovyWave currently uses a mixed reactive architecture with 69+ global mutables that suffer from multi-location mutations, circular dependencies, and difficult-to-trace state changes. This document proposes migrating to an Actor Model architecture where each mutable becomes an autonomous unit with clear data flow dependencies, eliminating the "globals modified from everywhere" antipattern.

## Current Architecture Analysis

### State Management Overview

The codebase contains **69+ static mutables** in `frontend/src/state.rs`:

```rust
// Examples of current global mutables
pub static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = lazy::default();
pub static SELECTED_VARIABLES: Lazy<MutableVec<SelectedVariable>> = lazy::default();
pub static EXPANDED_SCOPES: Lazy<Mutable<IndexSet<String>>> = lazy::default();
pub static TIMELINE_CURSOR_POSITION: Lazy<Mutable<f64>> = Lazy::new(|| Mutable::new(0.0));
```

### Identified Problems

#### 1. Multi-Location Mutations

**TRACKED_FILES** is modified from 8+ locations:
- `state.rs`: Direct `lock_mut()` calls
- `config/triggers.rs`: Config restoration
- `views.rs`: UI interactions
- `utils.rs`: File operations

```rust
// Current problematic pattern - mutations from everywhere
TRACKED_FILES.lock_mut().push_cloned(file);  // In state.rs
TRACKED_FILES.lock_mut().clear();             // In config.rs
TRACKED_FILES.lock_mut().set_cloned(i, file); // In views.rs
```

#### 2. Circular Dependencies & Race Conditions

The config system uses guard flags to prevent loops:
```rust
pub static CONFIG_INITIALIZATION_COMPLETE: Lazy<Mutable<bool>> = lazy::default();

// Guard pattern to prevent circular updates
if CONFIG_INITIALIZATION_COMPLETE.get() {
    save_config_to_backend();
}
```

Comments reveal removed reactive patterns due to infinite loops:
```rust
// "Removed reactive config sync due to infinite loops"
// "One-shot sync to prevent circular dependencies"
```

#### 3. Complex Signal Chains

Current signal chains with multiple handlers:
```rust
// Multiple handlers for same signal
SELECTED_VARIABLES.signal_vec_cloned().for_each_sync(|vars| {
    timeline_service_call(vars); // Handler 1
});

// Somewhere else in codebase
SELECTED_VARIABLES.signal_vec_cloned().for_each_sync(|vars| {
    timeline_service_call(vars); // Duplicate handler
});
```

#### 4. Manual Coordination

Explicit triggers and sync methods proliferate:
```rust
pub fn sync_globals_to_config() { /* manual sync */ }
pub fn apply_config_to_globals() { /* manual apply */ }
pub fn populate_ui_from_state() { /* manual populate */ }
```

### Existing Actor Pattern (Partial)

Interestingly, `TRACKED_FILES` already implements a partial actor model:

```rust
pub enum FileUpdateMessage {
    Add { tracked_file: TrackedFile },
    Update { file_id: String, new_state: FileState },
    Remove { file_id: String },
}

static FILE_UPDATE_QUEUE: Lazy<Mutable<Vec<FileUpdateMessage>>> = ...;

async fn process_file_update_message_sync(message: FileUpdateMessage) {
    // Single processor for all file mutations
    match message {
        FileUpdateMessage::Add { tracked_file } => { /* ... */ }
        FileUpdateMessage::Update { file_id, new_state } => { /* ... */ }
        FileUpdateMessage::Remove { file_id } => { /* ... */ }
    }
}
```

This proves the pattern works but needs expansion to other state domains.

## Proposed Actor Architecture

### Core Principles

1. **Single Owner**: Each mutable has exactly ONE actor that can modify it
2. **Message-Based**: All mutations happen through messages, never direct access
3. **Dependency-Driven**: Actors listen to signal dependencies and update accordingly
4. **Pure Reactive**: No manual sync/apply/populate methods
5. **Composable**: Complex state derives from simpler actor outputs

### Actor Types

#### 1. Pure Functional Actors
Stateless transformation of inputs to outputs:
```rust
struct TimelineCursorActor;

impl TimelineCursorActor {
    fn new() -> Self {
        Task::start(async {
            map_ref! {
                let mouse = MOUSE_POSITION.signal(),
                let keyboard = KEYBOARD_CURSOR.signal(),
                let config = CONFIG_CURSOR.signal() =>
                
                // Pure function: inputs → output
                let cursor_pos = compute_cursor_position(mouse, keyboard, config);
                TIMELINE_CURSOR_POSITION.set_neq(cursor_pos);
            }
        });
        Self
    }
}
```

#### 2. Stateful Actors
Maintain internal state and process messages:
```rust
struct VariableSelectionActor {
    message_queue: Mutable<Vec<SelectionMessage>>,
}

enum SelectionMessage {
    Add { variable: Signal, file_id: String, scope_id: String },
    Remove { unique_id: String },
    Clear,
    RestoreFromConfig { variables: Vec<SelectedVariable> },
}

impl VariableSelectionActor {
    async fn process_message(&self, msg: SelectionMessage) {
        match msg {
            SelectionMessage::Add { variable, file_id, scope_id } => {
                // Validate, deduplicate, update index
                let selected = create_selected_variable(variable, file_id, scope_id);
                if !SELECTED_VARIABLES_INDEX.lock_ref().contains(&selected.unique_id) {
                    SELECTED_VARIABLES.lock_mut().push_cloned(selected);
                    SELECTED_VARIABLES_INDEX.lock_mut().insert(selected.unique_id);
                }
            }
            // ... other messages
        }
    }
}
```

### Dependency Graph Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     External Inputs                          │
│  (Mouse Events, Keyboard, Backend Messages, Config Load)    │
└──────────┬──────────────────────────────────┬───────────────┘
           │                                  │
           ▼                                  ▼
┌──────────────────────┐          ┌──────────────────────┐
│  ConfigurationActor  │          │   FileSystemActor    │
│  ├─ theme            │          │   ├─ file_paths      │
│  ├─ dock_mode        │          │   ├─ loading_status  │
│  └─ panel_sizes      │          │   └─ file_tree       │
└──────────┬───────────┘          └──────────┬───────────┘
           │                                  │
           ▼                                  ▼
┌──────────────────────┐          ┌──────────────────────┐
│  PanelLayoutActor    │◄─────────┤  FileManagementActor │
│  ├─ panel_widths     │          │   ├─ tracked_files   │
│  └─ dock_state       │          │   └─ smart_labels    │
└──────────────────────┘          └──────────┬───────────┘
                                             │
           ┌─────────────────────────────────┴───────────┐
           ▼                                             ▼
┌──────────────────────┐          ┌──────────────────────┐
│ ScopeExpansionActor  │          │VariableSelectionActor│
│  └─ expanded_scopes  │          │  ├─ selected_vars    │
└──────────────────────┘          │  └─ var_index       │
                                  └──────────┬───────────┘
                                             │
                                             ▼
                                  ┌──────────────────────┐
                                  │  TimelineStateActor  │
                                  │  ├─ cursor_position  │
                                  │  ├─ zoom_level       │
                                  │  └─ visible_range    │
                                  └──────────┬───────────┘
                                             │
                                             ▼
                                  ┌──────────────────────┐
                                  │  SignalDataActor     │
                                  │  ├─ signal_values    │
                                  │  └─ transitions      │
                                  └──────────────────────┘
```

### Message Flow Examples

#### Example 1: User Selects a Variable

```rust
// Current (problematic) approach:
fn on_variable_click(var: Signal) {
    // Direct mutation from UI
    SELECTED_VARIABLES.lock_mut().push_cloned(var);
    // Manual trigger
    trigger_signal_value_queries();
    // Manual save
    save_selected_variables();
}

// Actor-based approach:
fn on_variable_click(var: Signal, file_id: String, scope_id: String) {
    // Just send a message
    VariableSelectionActor::send(SelectionMessage::Add { 
        variable: var,
        file_id,
        scope_id,
    });
    // Actor handles validation, deduplication, index update, 
    // config save, and triggering dependent actors
}
```

#### Example 2: Config Load

```rust
// Current approach with guard flags:
fn load_config() {
    let config = read_config_file();
    CONFIG_INITIALIZATION_COMPLETE.set(false); // Guard
    
    // Manual population
    TRACKED_FILES.lock_mut().replace_cloned(config.files);
    SELECTED_VARIABLES.lock_mut().replace_cloned(config.variables);
    EXPANDED_SCOPES.set(config.expanded);
    
    CONFIG_INITIALIZATION_COMPLETE.set(true); // Enable saves
}

// Actor-based approach:
fn load_config() {
    let config = read_config_file();
    
    // Send restore messages to each actor
    FileManagementActor::send(FileMessage::RestoreFromConfig { 
        files: config.files 
    });
    VariableSelectionActor::send(SelectionMessage::RestoreFromConfig { 
        variables: config.variables 
    });
    ScopeExpansionActor::send(ScopeMessage::RestoreFromConfig { 
        scopes: config.expanded 
    });
    // Each actor handles its own restoration logic
}
```

### Implementation Strategy

#### Phase 1: Extend FileManagementActor (Week 1)
- Build on existing message queue implementation
- Replace all 8+ `TRACKED_FILES.lock_mut()` locations
- Add smart labeling logic to actor

```rust
impl FileManagementActor {
    async fn process_message(&self, msg: FileMessage) {
        match msg {
            FileMessage::Add { path } => {
                let tracked_file = create_tracked_file(path);
                let smart_label = compute_smart_label(&tracked_file);
                tracked_file.smart_label = smart_label;
                TRACKED_FILES.lock_mut().push_cloned(tracked_file);
            }
            FileMessage::UpdateState { id, state } => {
                // Update and recompute labels if needed
            }
            // ... other messages
        }
        
        // Auto-trigger dependent actors
        self.notify_dependents();
    }
}
```

#### Phase 2: VariableSelectionActor (Week 2)
- High impact, clear boundaries
- Eliminate duplicate signal handlers
- Automatic config persistence

```rust
struct VariableSelectionActor {
    dependencies: Dependencies {
        tracked_files: TRACKED_FILES.signal_vec_cloned(),
        scope_expansion: EXPANDED_SCOPES.signal(),
    }
}

impl VariableSelectionActor {
    fn new() -> Self {
        // Listen to dependencies
        Task::start(async {
            map_ref! {
                let files = self.dependencies.tracked_files,
                let scopes = self.dependencies.scope_expansion =>
                
                // Revalidate selections when dependencies change
                self.validate_selections(files, scopes);
            }
        });
        
        Self { /* ... */ }
    }
}
```

#### Phase 3: ConfigurationActor (Week 3)
- Replace bidirectional config sync
- Eliminate guard flags
- Unified config flow

```rust
struct ConfigurationActor;

impl ConfigurationActor {
    fn new() -> Self {
        // Listen to all state that needs persistence
        Task::start(async {
            map_ref! {
                let files = TRACKED_FILES.signal_vec_cloned().to_signal_cloned(),
                let variables = SELECTED_VARIABLES.signal_vec_cloned().to_signal_cloned(),
                let scopes = EXPANDED_SCOPES.signal() =>
                
                // Debounced save (prevent spam)
                self.schedule_config_save(files, variables, scopes);
            }
        });
        
        Self
    }
    
    async fn schedule_config_save(&self, files: Vec<TrackedFile>, /*...*/) {
        // Cancel previous save task
        self.save_task_handle.set(None);
        
        // Schedule new save
        let handle = Task::start_droppable(async {
            Timer::sleep(1000).await; // 1 second debounce
            save_config_to_backend(files, /*...*/);
        });
        
        self.save_task_handle.set(Some(handle));
    }
}
```

#### Phase 4: TimelineStateActor (Week 4)
- Coordinate cursor, zoom, pan
- Unified timeline state management

### API Design

#### Actor Trait

```rust
trait Actor {
    type Message;
    type State;
    
    /// Process a single message
    async fn process_message(&self, msg: Self::Message);
    
    /// Get current state (read-only)
    fn state(&self) -> Self::State;
    
    /// Send a message to this actor
    fn send(msg: Self::Message) {
        self.queue.lock_mut().push(msg);
    }
}
```

#### Dependency Declaration

```rust
#[derive(Dependencies)]
struct VariableSelectionDeps {
    #[dependency(signal_vec)]
    tracked_files: MutableVec<TrackedFile>,
    
    #[dependency(signal)]
    expanded_scopes: Mutable<IndexSet<String>>,
    
    #[dependency(signal)]
    search_filter: Mutable<String>,
}
```

#### Actor Registration

```rust
pub fn initialize_actors() {
    // Create actor instances
    let file_actor = FileManagementActor::new();
    let variable_actor = VariableSelectionActor::new();
    let config_actor = ConfigurationActor::new();
    
    // Register for global access
    ACTORS.register("files", file_actor);
    ACTORS.register("variables", variable_actor);
    ACTORS.register("config", config_actor);
    
    // Start processing
    ACTORS.start_all();
}
```

### Testing Strategy

#### Unit Testing Actors

```rust
#[cfg(test)]
mod tests {
    #[test]
    async fn test_variable_selection_deduplication() {
        let actor = VariableSelectionActor::new();
        
        // Send duplicate add messages
        actor.send(SelectionMessage::Add { 
            variable: test_signal(),
            file_id: "file1",
            scope_id: "scope1",
        });
        actor.send(SelectionMessage::Add { 
            variable: test_signal(),
            file_id: "file1", 
            scope_id: "scope1",
        });
        
        // Wait for processing
        Timer::sleep(100).await;
        
        // Should only have one variable
        assert_eq!(SELECTED_VARIABLES.lock_ref().len(), 1);
    }
}
```

#### Integration Testing

```rust
#[test]
async fn test_config_restore_flow() {
    // Initialize actors
    initialize_actors();
    
    // Load config
    ConfigurationActor::send(ConfigMessage::Load { 
        path: "test.novywave" 
    });
    
    // Wait for cascade
    Timer::sleep(500).await;
    
    // Verify state restored
    assert_eq!(TRACKED_FILES.lock_ref().len(), 3);
    assert_eq!(SELECTED_VARIABLES.lock_ref().len(), 5);
    assert!(EXPANDED_SCOPES.lock_ref().contains("scope1"));
}
```

### Migration Path

#### Compatibility Layer

During migration, maintain backwards compatibility:

```rust
// Old API (deprecated)
pub fn add_tracked_file(path: String, state: FileState) {
    #[deprecated(note = "Use FileManagementActor::send() instead")]
    FileManagementActor::send(FileMessage::Add { 
        path,
        initial_state: state,
    });
}

// New API
FileManagementActor::send(FileMessage::Add { 
    path: "file.vcd",
    initial_state: FileState::Loading,
});
```

#### Gradual Migration

1. **Start with leaf nodes**: Actors with no dependents
2. **Move up dependency graph**: Actors that depend on migrated ones
3. **Keep old patterns temporarily**: For unmigrated code
4. **Final cleanup**: Remove all direct mutations

### Performance Considerations

#### Benefits

1. **Reduced Lock Contention**: Single writer per mutable
2. **Predictable Updates**: Clear causality chain
3. **Better Caching**: Actors can maintain local caches
4. **Parallel Processing**: Independent actors run concurrently

#### Potential Issues

1. **Message Overhead**: Serialization/deserialization cost
   - **Mitigation**: Use zero-copy messages where possible
   
2. **Latency**: Message queue processing delay
   - **Mitigation**: Priority queues for UI-critical messages
   
3. **Memory**: Actor state + message queues
   - **Mitigation**: Bounded queues with backpressure

### Monitoring & Debugging

#### Actor Inspector

```rust
impl Actor {
    fn inspect(&self) -> ActorStats {
        ActorStats {
            message_queue_length: self.queue.lock_ref().len(),
            messages_processed: self.processed_count.get(),
            average_processing_time: self.total_time.get() / self.processed_count.get(),
            last_message_time: self.last_message_time.get(),
        }
    }
}
```

#### Debug Mode

```rust
#[cfg(debug_assertions)]
fn debug_message(actor: &str, msg: &impl Debug) {
    zoon::println!("🎭 [{}] Processing: {:?}", actor, msg);
}
```

#### Deadlock Detection

```rust
struct DeadlockDetector {
    dependency_graph: HashMap<ActorId, Vec<ActorId>>,
}

impl DeadlockDetector {
    fn check_for_cycles(&self) -> Option<Vec<ActorId>> {
        // Detect circular dependencies
    }
}
```

## Open Questions

### 1. Granularity

**Question**: Should every mutable have its own actor, or should we group related state?

**Considerations**:
- **Fine-grained**: Better isolation, more boilerplate
- **Coarse-grained**: Less overhead, potential coupling

**Recommendation**: Start coarse (one actor per domain), refine as needed.

### 2. Message Persistence

**Question**: Should we persist messages for replay/debugging?

**Considerations**:
- **Benefits**: Time-travel debugging, crash recovery
- **Costs**: Memory/storage overhead

**Recommendation**: Optional debug mode with ring buffer.

### 3. Error Handling

**Question**: How should actors handle errors?

**Options**:
1. **Supervisor Pattern**: Parent actors restart failed children
2. **Error Messages**: Errors become messages to error actor
3. **Panic**: Let it crash, restart everything

**Recommendation**: Error messages for recoverable, panic for unrecoverable.

### 4. Signal vs Message

**Question**: When to use Zoon signals vs actor messages?

**Guidelines**:
- **Signals**: Read-only derived state, UI bindings
- **Messages**: State mutations, commands, events

### 5. Batch Operations

**Question**: How to handle bulk updates efficiently?

**Options**:
1. **Batch Messages**: `AddMultiple { items: Vec<T> }`
2. **Transaction Messages**: `BeginBatch`, `EndBatch`
3. **Automatic Batching**: Coalesce messages in queue

**Recommendation**: Explicit batch messages for clarity.

## Success Metrics

### Quantitative

1. **Lock Contention**: Reduce mutex conflicts by 90%
2. **Code Reduction**: Eliminate 50+ `lock_mut()` calls
3. **Bug Reduction**: Fewer state synchronization bugs
4. **Performance**: No regression in UI responsiveness

### Qualitative

1. **Traceability**: Clear path from action to state change
2. **Testability**: Isolated actors easier to test
3. **Maintainability**: New developers understand flow quickly
4. **Debuggability**: Actor inspector shows system state

## Risks & Mitigations

### Risk 1: Migration Complexity

**Risk**: Large refactor disrupts development
**Mitigation**: Incremental migration with compatibility layer

### Risk 2: Performance Regression

**Risk**: Message passing slower than direct mutation
**Mitigation**: Profile critical paths, optimize hot spots

### Risk 3: Over-Engineering

**Risk**: Too many actors, too much indirection
**Mitigation**: Start simple, add actors only when needed

## Conclusion

The Actor Model architecture addresses NovyWave's current pain points:
- Eliminates multi-location mutations
- Removes circular dependencies
- Simplifies state management
- Improves debuggability

By building on the existing partial implementation and migrating incrementally, we can transform the codebase without disrupting development. The key is to start with high-impact, well-bounded actors and expand gradually.

## Appendix: Code Examples

### Complete VariableSelectionActor Implementation

```rust
use zoon::*;
use std::collections::HashSet;

pub struct VariableSelectionActor {
    message_queue: Mutable<Vec<SelectionMessage>>,
    processing_handle: Option<TaskHandle>,
}

#[derive(Debug, Clone)]
pub enum SelectionMessage {
    Add { 
        variable: shared::Signal, 
        file_id: String, 
        scope_id: String 
    },
    Remove { 
        unique_id: String 
    },
    Clear,
    Toggle { 
        variable: shared::Signal,
        file_id: String,
        scope_id: String,
    },
    RestoreFromConfig { 
        variables: Vec<shared::SelectedVariable> 
    },
}

impl VariableSelectionActor {
    pub fn new() -> Self {
        let queue = Mutable::new(Vec::new());
        let queue_clone = queue.clone();
        
        // Start message processor
        let handle = Task::start_droppable(async move {
            loop {
                // Take messages from queue
                let messages = {
                    let mut q = queue_clone.lock_mut();
                    if q.is_empty() {
                        drop(q);
                        Timer::sleep(10).await;
                        continue;
                    }
                    std::mem::take(&mut *q)
                };
                
                // Process each message
                for msg in messages {
                    Task::next_macro_tick().await; // Yield between messages
                    Self::process_message_internal(msg).await;
                }
            }
        });
        
        Self {
            message_queue: queue,
            processing_handle: Some(handle),
        }
    }
    
    pub fn send(msg: SelectionMessage) {
        // Get global actor instance
        VARIABLE_SELECTION_ACTOR.with(|actor| {
            actor.message_queue.lock_mut().push(msg);
        });
    }
    
    async fn process_message_internal(msg: SelectionMessage) {
        match msg {
            SelectionMessage::Add { variable, file_id, scope_id } => {
                // Create selected variable
                let selected = create_selected_variable(variable, &file_id, &scope_id);
                
                // Check for duplicates
                let mut index = SELECTED_VARIABLES_INDEX.lock_mut();
                if !index.contains(&selected.unique_id) {
                    index.insert(selected.unique_id.clone());
                    drop(index); // Release lock before next mutation
                    
                    SELECTED_VARIABLES.lock_mut().push_cloned(selected);
                    
                    // Notify dependent actors
                    TimelineStateActor::send(TimelineMessage::VariablesChanged);
                    SignalDataActor::send(SignalDataMessage::RefreshValues);
                }
            }
            
            SelectionMessage::Remove { unique_id } => {
                SELECTED_VARIABLES.lock_mut().retain(|v| v.unique_id != unique_id);
                SELECTED_VARIABLES_INDEX.lock_mut().remove(&unique_id);
                
                // Notify dependent actors
                TimelineStateActor::send(TimelineMessage::VariablesChanged);
            }
            
            SelectionMessage::Clear => {
                SELECTED_VARIABLES.lock_mut().clear();
                SELECTED_VARIABLES_INDEX.lock_mut().clear();
                
                // Notify dependent actors  
                TimelineStateActor::send(TimelineMessage::VariablesChanged);
            }
            
            SelectionMessage::Toggle { variable, file_id, scope_id } => {
                let unique_id = format!("{}|{}|{}", file_id, scope_id, variable.name);
                
                if SELECTED_VARIABLES_INDEX.lock_ref().contains(&unique_id) {
                    Self::process_message_internal(
                        SelectionMessage::Remove { unique_id }
                    ).await;
                } else {
                    Self::process_message_internal(
                        SelectionMessage::Add { variable, file_id, scope_id }
                    ).await;
                }
            }
            
            SelectionMessage::RestoreFromConfig { variables } => {
                // Clear and restore
                SELECTED_VARIABLES.lock_mut().replace_cloned(variables.clone());
                
                let index: HashSet<String> = variables.iter()
                    .map(|v| v.unique_id.clone())
                    .collect();
                *SELECTED_VARIABLES_INDEX.lock_mut() = index;
                
                // Notify dependent actors
                TimelineStateActor::send(TimelineMessage::VariablesChanged);
                SignalDataActor::send(SignalDataMessage::RefreshValues);
            }
        }
    }
}

// Global instance
thread_local! {
    static VARIABLE_SELECTION_ACTOR: VariableSelectionActor = VariableSelectionActor::new();
}
```

### Complete TimelineCursorActor Implementation

```rust
pub struct TimelineCursorActor;

impl TimelineCursorActor {
    pub fn new() -> Self {
        // Pure reactive actor - no message queue needed
        Task::start(async {
            map_ref! {
                // Listen to all cursor position inputs
                let mouse_time = MOUSE_TIME_POSITION.signal(),
                let keyboard_left = IS_CURSOR_MOVING_LEFT.signal(),
                let keyboard_right = IS_CURSOR_MOVING_RIGHT.signal(),
                let config_position = CONFIG_CURSOR_POSITION.signal(),
                let timeline_range = TIMELINE_VISIBLE_RANGE.signal() =>
                
                // Compute final cursor position
                let cursor_pos = Self::compute_cursor_position(
                    *mouse_time,
                    *keyboard_left,
                    *keyboard_right,
                    *config_position,
                    timeline_range,
                );
                
                // Update only if changed
                TIMELINE_CURSOR_POSITION.set_neq(cursor_pos);
            }
        });
        
        Self
    }
    
    fn compute_cursor_position(
        mouse_time: f32,
        moving_left: bool,
        moving_right: bool,
        config_pos: f64,
        range: (f32, f32),
    ) -> f64 {
        // Priority: keyboard > mouse > config
        if moving_left {
            let current = TIMELINE_CURSOR_POSITION.get();
            (current - 0.1).max(range.0 as f64)
        } else if moving_right {
            let current = TIMELINE_CURSOR_POSITION.get();
            (current + 0.1).min(range.1 as f64)
        } else if mouse_time > 0.0 {
            mouse_time as f64
        } else {
            config_pos
        }
    }
}
```

### Actor System Initialization

```rust
pub fn initialize_actor_system() {
    // Create all actors
    let file_actor = FileManagementActor::new();
    let variable_actor = VariableSelectionActor::new();
    let scope_actor = ScopeExpansionActor::new();
    let timeline_actor = TimelineStateActor::new();
    let signal_data_actor = SignalDataActor::new();
    let config_actor = ConfigurationActor::new();
    
    // Set up dependency relationships
    file_actor.on_change(|| {
        variable_actor.send(SelectionMessage::ValidateSelections);
        scope_actor.send(ScopeMessage::ValidateExpansions);
    });
    
    variable_actor.on_change(|| {
        timeline_actor.send(TimelineMessage::VariablesChanged);
        signal_data_actor.send(SignalDataMessage::RefreshValues);
    });
    
    // Start all actors
    zoon::println!("🎭 Actor system initialized");
}
```