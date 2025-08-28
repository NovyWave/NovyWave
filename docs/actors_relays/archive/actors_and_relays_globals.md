# Actor+Relay Architecture: Global State Patterns

> **⚠️ BRIDGE DOCUMENTATION**: This file contains global state patterns for Actor+Relay architecture. These patterns serve as a bridge between traditional MoonZoon globals and idiomatic local state. For production applications, prefer the local state patterns in `actors_and_relays.md`.

This document covers when and how to use global Actor+Relay patterns, including migration strategies from global Mutables and architectural guidelines for global state management.

## Quick Reference: Local vs Global State

### **Default Choice: Local State**
Start with local state using Actor+Relay within components. Only use globals when local becomes awkward.

### **Use Global State When:**
- State is naturally shared across entire app (files, theme, settings)
- Multiple unrelated components need the same state  
- Component tree becomes unwieldy with parameter passing
- Singleton services (connection managers, configuration, logging)

## Global State Guidelines

### ✅ Use globals only for truly app-wide state:

```rust
// File management, theme, timeline position - naturally global
static TRACKED_FILES: Lazy<ActorVec<TrackedFile>> = lazy::default();
static CURRENT_THEME: Lazy<Actor<Theme>> = lazy::default();
static TIMELINE_POSITION: Lazy<Actor<f64>> = lazy::default();
```

### Examples of Appropriate Global State:
- **File management systems**: Files opened across the application
- **Theme and configuration**: Settings that affect all UI components
- **Timeline/viewport position**: Navigation state shared by multiple panels
- **Connection state**: WebSocket or HTTP connection managers
- **User authentication**: Login state accessed throughout the app

### Examples of Inappropriate Global State:
- **Component-specific state**: Dialog open/closed, form input values
- **Temporary UI state**: Hover states, loading spinners for individual operations
- **Local collections**: Data specific to one component or view

## Migration Patterns

### Pattern 1: Global Message Queue → Structural Relays

**Before (Global Mutables):**
```rust
// Scattered global state with unclear responsibilities
pub static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = lazy::default();
pub static LOADING_FILES: Lazy<MutableVec<LoadingFile>> = lazy::default();
pub static FILE_OPERATIONS: Lazy<MutableVec<FileOperation>> = lazy::default();

// Unclear who can modify what, when, and why
fn add_file(path: String) {
    TRACKED_FILES.lock_mut().push_cloned(TrackedFile::new(path.clone()));
    LOADING_FILES.lock_mut().push_cloned(LoadingFile::new(path, LoadingState::Pending));
    FILE_OPERATIONS.lock_mut().push_cloned(FileOperation::Add(path));
}
```

**After (Global Actor+Relay):**
```rust
#[derive(Clone)]
struct FileManager {
    pub files: ActorVec<TrackedFile>,
    pub add_file: Relay<String>,
    pub remove_file: Relay<String>,
    pub update_file_state: Relay<(String, FileState)>,
}

impl Default for FileManager {
    fn default() -> Self {
        let (add_file, mut add_stream) = relay();
        let (remove_file, mut remove_stream) = relay();
        let (update_file_state, mut update_stream) = relay();
        
        let files = ActorVec::new(vec![], async move |files_vec| {
            loop {
                select! {
                    Some(path) = add_stream.next() => {
                        let file = TrackedFile::new(path);
                        files_vec.lock_mut().push_cloned(file);
                    }
                    Some(path) = remove_stream.next() => {
                        files_vec.lock_mut().retain(|f| f.path != path);
                    }
                    Some((path, state)) = update_stream.next() => {
                        if let Some(file) = files_vec.lock_mut().iter_mut().find(|f| f.path == path) {
                            file.state = state;
                        }
                    }
                }
            }
        });
        
        FileManager { files, add_file, remove_file, update_file_state }
    }
}

// Global instance - properly encapsulated
static FILE_MANAGER: Lazy<FileManager> = lazy::default();

// Clear, controlled API
pub fn add_file(path: String) {
    FILE_MANAGER.add_file.send(path);
}
```

### Pattern 2: Global Mutables → Domain Structs

**Before (Multiple Global Mutables):**
```rust
// Global state with uncontrolled access
pub static SELECTED_VARIABLES: Lazy<MutableVec<SelectedVariable>> = lazy::default();

// Multiple mutation points - impossible to track
SELECTED_VARIABLES.lock_mut().clear();  // Location A
SELECTED_VARIABLES.lock_mut().push_cloned(var);  // Location B
SELECTED_VARIABLES.lock_mut().retain(|v| v.id != target_id);  // Location C
```

**After (Global Actor+Relay Domain Struct):**
```rust
#[derive(Clone)]
struct VariableSelection {
    pub variables: ActorVec<SelectedVariable>,
    pub select: Relay<SelectedVariable>,
    pub deselect: Relay<String>,
    pub clear_all: Relay,
}

impl Default for VariableSelection {
    fn default() -> Self {
        let (select, mut select_stream) = relay();
        let (deselect, mut deselect_stream) = relay();
        let (clear_all, mut clear_stream) = relay();
        
        let variables = ActorVec::new(vec![], async move |vars_vec| {
            loop {
                select! {
                    Some(var) = select_stream.next() => {
                        // Avoid duplicates
                        let mut vars = vars_vec.lock_mut();
                        if !vars.iter().any(|v| v.id == var.id) {
                            vars.push_cloned(var);
                        }
                    }
                    Some(target_id) = deselect_stream.next() => {
                        vars_vec.lock_mut().retain(|v| v.id != target_id);
                    }
                    Some(()) = clear_stream.next() => {
                        vars_vec.lock_mut().clear();
                    }
                }
            }
        });
        
        VariableSelection { variables, select, deselect, clear_all }
    }
}

// Global instance with clear API
static VARIABLE_SELECTION: Lazy<VariableSelection> = lazy::default();

// Controlled mutations through events
pub fn select_variable(var: SelectedVariable) {
    VARIABLE_SELECTION.select.send(var);
}

pub fn deselect_variable(id: String) {
    VARIABLE_SELECTION.deselect.send(id);
}

pub fn clear_selection() {
    VARIABLE_SELECTION.clear_all.send(());
}
```

## Global Actor+Relay Benefits

### 1. **🔒 Controlled Mutations**
All state changes go through defined relays - no direct field access.

### 2. **📡 Event Traceability**
Every state change is a typed event that can be logged, debugged, and audited.

### 3. **⚡ Race-Condition Prevention**
Atomic relay operations eliminate get/set race conditions.

### 4. **🎯 Clear Responsibilities**
Domain structs define exactly who can change what state.

### 5. **🧪 Better Testing**
Events can be sent programmatically for testing scenarios.

## Implementation Roadmap: Global Migration

### Phase 1: Core Infrastructure (Week 1)
- [ ] Implement Actor, Relay, ActorVec base types
- [ ] Create Atom helper for basic local state
- [ ] Convert critical UI components to use Atom
- [ ] Verify no existing functionality broken

### Phase 2: File Management Migration (Week 2)
- [ ] Convert `TRACKED_FILES` to `FileManager` ActorVec
- [ ] Update all file operation call sites to use relays
- [ ] Migrate file loading state management
- [ ] Verify no recursive locks remain

### Phase 3: Critical State Migration (Week 3-4)
- [ ] Convert `SELECTED_VARIABLES` to `VariableSelection` ActorVec
- [ ] Convert `TIMELINE_CURSOR_POSITION` to Actor<f64>
- [ ] Convert search/filter state to Actors
- [ ] Convert config system to type-safe Actor

### Phase 4: UI State Cleanup (Week 5)
- [ ] Replace remaining raw Mutables with Atom
- [ ] Convert dialog management to unified Actor system
- [ ] Migrate viewport/scrolling state management
- [ ] Performance testing and optimization

## Global State Anti-Patterns to Avoid

### ❌ Multiple Global Sources for Related State
```rust
// BAD: Related state scattered across multiple globals
static LOADING_FILES: Lazy<MutableVec<String>> = lazy::default();
static FILE_ERRORS: Lazy<MutableVec<(String, String)>> = lazy::default();
static FILE_PROGRESS: Lazy<MutableBTreeMap<String, f32>> = lazy::default();

// GOOD: Unified domain struct
static FILE_MANAGER: Lazy<FileManager> = lazy::default();
```

### ❌ Global State for Component-Specific Concerns
```rust
// BAD: Global state for component-specific UI
static DIALOG_OPEN: Lazy<Mutable<bool>> = lazy::default();
static HOVER_STATE: Lazy<Mutable<bool>> = lazy::default();

// GOOD: Local state within component
fn my_component() -> impl Element {
    let dialog_state = Atom::new(false);
    let hover_state = Atom::new(false);
    // Use local state...
}
```

### ❌ Direct Global Mutable Access
```rust
// BAD: Direct mutations bypass event system
static COUNTERS: Lazy<MutableVec<i32>> = lazy::default();
COUNTERS.lock_mut().push(42);  // No traceability

// GOOD: Controlled mutations through relays
static COUNTER_COLLECTION: Lazy<CounterCollection> = lazy::default();
COUNTER_COLLECTION.add_counter.send(42);  // Traceable event
```

## Global Testing Strategies

### Testing Global State Changes
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_global_file_management() {
        // Reset global state
        FILE_MANAGER.clear_all.send(());
        
        // Test file addition
        FILE_MANAGER.add_file.send("test.txt".to_string());
        
        let mut files_stream = FILE_MANAGER.files.signal_vec_cloned().len().to_stream();
        assert_eq!(files_stream.next().await.unwrap(), 1);
        
        // Test file removal
        FILE_MANAGER.remove_file.send("test.txt".to_string());
        assert_eq!(files_stream.next().await.unwrap(), 0);
    }
    
    #[async_test]
    async fn test_variable_selection() {
        // Reset global state
        VARIABLE_SELECTION.clear_all.send(());
        
        let var = SelectedVariable::new("test_var");
        VARIABLE_SELECTION.select.send(var.clone());
        
        let mut vars_stream = VARIABLE_SELECTION.variables.signal_vec_cloned().to_signal_cloned().to_stream();
        let vars = vars_stream.next().await.unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].id, "test_var");
    }
}
```

### Isolating Global State in Tests
```rust
// Helper for test isolation
async fn reset_global_state() {
    FILE_MANAGER.clear_all.send(());
    VARIABLE_SELECTION.clear_all.send(());
    CURRENT_THEME.reset.send(());
    // Wait for state to settle
    Timer::sleep(10).await;
}

#[async_test]
async fn test_with_clean_state() {
    reset_global_state().await;
    // Test logic with known clean state
}
```

## Performance Considerations

### Global State Access Patterns
```rust
// ✅ EFFICIENT: Cache signal references for frequent access
fn timeline_component() -> impl Element {
    let position_signal = TIMELINE_POSITION.signal(); // Cache once
    
    Column::new()
        .item_signal(position_signal.map(|pos| format!("Position: {}", pos)))
        .item_signal(position_signal.map(|pos| render_cursor(pos)))
}

// ❌ INEFFICIENT: Create new signal references repeatedly
fn timeline_component_bad() -> impl Element {
    Column::new()
        .item_signal(TIMELINE_POSITION.signal().map(|pos| format!("Position: {}", pos)))
        .item_signal(TIMELINE_POSITION.signal().map(|pos| render_cursor(pos)))
}
```

### Global Collection Updates
```rust
// ✅ EFFICIENT: Batch related updates
pub fn update_multiple_files(updates: Vec<(String, FileState)>) {
    for (path, state) in updates {
        FILE_MANAGER.update_file_state.send((path, state));
    }
}

// ❌ INEFFICIENT: Individual update calls
pub fn update_multiple_files_bad(updates: Vec<(String, FileState)>) {
    for (path, state) in updates {
        // Each call creates separate signal emissions
        update_single_file(path, state);
    }
}
```

## Migration Decision Tree

### When to Keep Global State:
- [ ] Used by 3+ unrelated components
- [ ] Represents true application-wide concerns (files, theme, config)
- [ ] Singleton services (connections, caches)
- [ ] Cross-cutting functionality (logging, metrics)

### When to Convert to Local State:
- [ ] Used by 1-2 related components
- [ ] Component-specific UI state (dialogs, forms, temporary data)
- [ ] Can be easily passed down component tree
- [ ] Testing would be simpler with isolation

### Migration Priority:
1. **High Priority**: State causing recursive locks or race conditions
2. **Medium Priority**: Frequently accessed state with unclear ownership
3. **Low Priority**: Stable global state working correctly

## Conclusion

Global Actor+Relay patterns serve as a bridge between traditional MoonZoon global Mutables and idiomatic local state management. Use these patterns sparingly and only when local state becomes unwieldy.

**Key principle**: Start with local state by default. Only use global patterns when you have genuine app-wide state that needs to be shared across unrelated components.

For most use cases, prefer the local state patterns documented in `actors_and_relays.md`.