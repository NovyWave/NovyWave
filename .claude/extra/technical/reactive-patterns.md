# Reactive Programming Patterns & Debugging Guide

This guide captures proven reactive programming patterns and debugging techniques learned from NovyWave development, including specific solutions to infinite loops, signal chain issues, and initialization problems.

## Core Reactive Patterns

### Signal Chain Composition

#### Basic Signal Combinations
```rust
// Combine multiple signals into one computation
map_ref! {
    let scope_id = SELECTED_SCOPE_ID.signal_ref(|id| id.clone()),
    let search_filter = VARIABLES_SEARCH_FILTER.signal_cloned(),
    let tracked_files = TRACKED_FILES.signal_vec_cloned().to_signal_cloned() =>
    {
        if let Some(scope_id) = scope_id {
            get_variables_from_tracked_files(&scope_id)
        } else {
            Vec::new()
        }
    }
}
```

#### Dynamic Element Creation with Type Unification
```rust
// Use .into_element() for type unification in conditional signals
.item_signal(content_signal.map(|content| {
    match content {
        ContentType::A => element_a().into_element(),  // ⚠️ .into_element() required
        ContentType::B => element_b().into_element(),
    }
}))
```

#### Checkbox State Synchronization
```rust
// ❌ Wrong: Static checkbox state
CheckboxBuilder::new()
    .checked(false) // Always unchecked

// ✅ Correct: Reactive checkbox state  
.label_signal(selected_items.signal_ref({
    let item_id = item_id.clone();
    move |selected| {
        CheckboxBuilder::new()
            .size(CheckboxSize::Small)
            .checked(selected.contains(&item_id))
            .build()
    }
}))
```

### Derived Signal Patterns

#### Safe Derived Signals (Prevents Circular Dependencies)
```rust
// ✅ Good: Derived signal doesn't modify source
pub static SMART_LABELS: Lazy<Mutable<HashMap<String, String>>> = Lazy::new(|| {
    let labels = Mutable::new(HashMap::new());
    
    Task::start({
        let labels = labels.clone();
        async move {
            TRACKED_FILES.signal_vec_cloned()
                .to_signal_cloned()
                .for_each(move |files| {
                    let labels = labels.clone();
                    async move {
                        let paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
                        let smart_labels = shared::generate_smart_labels(&paths);
                        labels.set_neq(smart_labels);  // Only updates derived state
                    }
                })
                .await;
        }
    });
    
    labels
});
```

#### Dangerous Patterns (Avoid)
```rust
// ❌ Bad: Derived signal modifies source (causes loops)
TRACKED_FILES.signal_vec_cloned().for_each_sync(move |files| {
    // Update smart labels
    for file in files {
        let mut file_mut = TRACKED_FILES.lock_mut();
        file_mut[0].smart_label = "new_label".to_string(); // Triggers signal again!
    }
});
```

## Initialization Patterns

### One-Shot Configuration Loading
```rust
// ✅ Correct: One-time initialization that doesn't create loops
pub fn setup_one_time_config_sync() {
    sync_file_management_from_config();
    sync_ui_state_from_config();
    sync_panel_layout_from_config();
    sync_timeline_from_config();
    sync_session_state_from_config();
    sync_selection_from_config();
}

fn sync_file_management_from_config() {
    let file_paths = config_store().session.lock_ref().opened_files.lock_ref().to_vec();
    
    if !file_paths.is_empty() {
        let current_files = TRACKED_FILES.lock_ref().to_vec();
        let mut new_tracked_files = Vec::new();
        
        for file_path in file_paths {
            // ✅ Preserve existing file states (prevents reset loops)
            if let Some(existing_file) = current_files.iter().find(|f| f.path == file_path) {
                new_tracked_files.push(existing_file.clone());
            } else {
                let new_file = create_tracked_file(file_path.clone(), FileState::Loading(LoadingStatus::Starting));
                new_tracked_files.push(new_file);
                
                // Send backend message for new files only
                Task::start({
                    let path = file_path.clone();
                    async move {
                        let _ = CurrentPlatform::send_message(UpMsg::LoadWaveformFile(path)).await;
                    }
                });
            }
        }
        
        // ✅ Only update if actually different
        let current_paths: Vec<String> = current_files.iter().map(|f| f.path.clone()).collect();
        let new_paths: Vec<String> = new_tracked_files.iter().map(|f| f.path.clone()).collect();
        
        if current_paths != new_paths {
            TRACKED_FILES.lock_mut().replace_cloned(new_tracked_files);
        }
    }
}
```

### Reactive Initialization with Signal Dependencies
```rust
// ✅ Ensuring UI reacts to file loading state changes
.child_signal(
    map_ref! {
        let selected_scope_id = SELECTED_SCOPE_ID.signal_ref(|id| id.clone()),
        let search_filter = VARIABLES_SEARCH_FILTER.signal_cloned(),
        let _tracked_files = TRACKED_FILES.signal_vec_cloned().to_signal_cloned() => // Critical for startup
        {
            if let Some(scope_id) = selected_scope_id {
                let variables = get_variables_from_tracked_files(&scope_id);
                virtual_variables_list(variables, search_filter.clone()).into_element()
            } else {
                empty_state_element().into_element()
            }
        }
    }
)
```

## Debugging Reactive Issues

### Identifying Infinite Loops

#### Console Log Patterns to Watch For:
```
🔨 [TreeView] RENDERING tree item: ... (thousands of identical logs)
🏷️ [DEBUG] Computing smart labels for N files (repeated rapidly)
🌲 [DEBUG] TreeView rendering with N files (excessive frequency)
```

#### Loop Detection Technique:
```rust
// Add counters to identify loops
static RENDER_COUNTER: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(0));

fn debug_render_call() {
    let count = RENDER_COUNTER.get() + 1;
    RENDER_COUNTER.set(count);
    if count % 100 == 0 {
        zoon::println!("⚠️ POTENTIAL LOOP: Render called {} times", count);
    }
}
```

### Common Issues & Solutions

#### Issue: Integer Overflow in Calculations
```rust
// ❌ Problem: Subtraction can underflow
let cleared_count = old_count - selected.len(); // Panics if selected.len() > old_count

// ✅ Solution: Use saturating arithmetic
let cleared_count = old_count.saturating_sub(selected.len());

// Or count the specific items being cleared
let old_scope_count = selected.iter().filter(|id| id.starts_with("scope_")).count();
let cleared_count = old_scope_count.saturating_sub(1);
```

#### Issue: Missing Signal Dependencies
```rust
// ❌ Problem: Variables panel doesn't update when files finish loading
map_ref! {
    let selected_scope_id = SELECTED_SCOPE_ID.signal_ref(|id| id.clone()),
    let search_filter = VARIABLES_SEARCH_FILTER.signal_cloned() =>
    // Missing TRACKED_FILES dependency - won't update when files load
}

// ✅ Solution: Add all relevant signal dependencies
map_ref! {
    let selected_scope_id = SELECTED_SCOPE_ID.signal_ref(|id| id.clone()),
    let search_filter = VARIABLES_SEARCH_FILTER.signal_cloned(),
    let _tracked_files = TRACKED_FILES.signal_vec_cloned().to_signal_cloned() => // Now reacts to file changes
}
```

#### Issue: Checkbox Signal Type Errors
```rust
// ❌ Problem: MutableSignalRef doesn't implement Clone
CheckboxBuilder::new()
    .checked_signal(selected_items.signal_ref({
        let item_id = item_id.clone();
        move |selected| selected.contains(&item_id)
    })) // Error: Clone not satisfied

// ✅ Solution: Use label_signal to recreate checkbox
.label_signal(selected_items.signal_ref({
    let item_id = item_id.clone();
    move |selected| {
        CheckboxBuilder::new()
            .size(CheckboxSize::Small)
            .checked(selected.contains(&item_id))
            .build()
    }
}))
```

### Debugging Workflow

1. **Identify the Loop Source**
   - Look for excessive console logging patterns
   - Add render counters to suspicious signal chains
   - Use browser dev tools to pause on repeated operations

2. **Trace Signal Dependencies**
   - Map out which signals trigger which updates
   - Look for circular dependencies (A→B→A)
   - Identify missing signal dependencies

3. **Check State Mutation Patterns**
   - Ensure derived signals don't modify their source
   - Look for bidirectional reactive flows
   - Verify state preservation during updates

4. **Test Initialization Order**
   - Check if issues only occur on app start
   - Verify config loading completes before UI rendering
   - Ensure all required signal dependencies are present

## Advanced Patterns

### Actor Model for Complex State
```rust
// For complex state with multiple interdependencies
#[derive(Debug, Clone)]
pub enum FileMessage {
    Add { path: String, state: FileState },
    UpdateState { id: String, state: FileState },
    Remove { id: String },
}

async fn process_file_message(message: FileMessage) {
    match message {
        FileMessage::Add { path, state } => {
            let mut files = TRACKED_FILES.lock_mut();
            files.push_cloned(TrackedFile::new(path, state));
        }
        FileMessage::UpdateState { id, state } => {
            let mut files = TRACKED_FILES.lock_mut();
            if let Some(file) = files.iter_mut().find(|f| f.id == id) {
                file.state = state;
            }
        }
        FileMessage::Remove { id } => {
            let mut files = TRACKED_FILES.lock_mut();
            files.retain(|f| f.id != id);
        }
    }
}

Task::start(async {
    loop {
        let messages = take_pending_messages();
        for message in messages {
            Task::next_macro_tick().await; // Yield between messages
            process_file_message(message).await;
        }
    }
});
```

### Unidirectional Data Flow
```rust
// ✅ Recommended: One-way data flow prevents loops
// Config → State → UI (one direction only)

// Config loads once
async fn initialize_from_config() {
    let config = load_config().await;
    setup_state_from_config_once(config);
    setup_state_to_config_persistence(); // Only state → config, not config → state
}

// State changes persist to config
fn setup_state_to_config_persistence() {
    Task::start(TRACKED_FILES.signal_vec_cloned().for_each(|files| async move {
        let paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
        config_store().session.lock_mut().opened_files.lock_mut().replace_cloned(paths);
        save_config_to_backend();
    }));
}
```

### Performance Optimization Patterns
```rust
// Deduplication to prevent unnecessary updates
TIMELINE_CURSOR_POSITION.signal().dedupe().for_each_sync(|pos| expensive_update(pos));

// Debouncing for expensive operations - see reference.md for modern Actor+Relay pattern
// ❌ DEPRECATED: Use Actor+Relay nested select! pattern instead
let debounce_handle: Mutable<Option<TaskHandle<()>>> = Mutable::new(None);
signal.for_each_sync(move |_| {
    debounce_handle.set(None); // Cancel previous
    let handle = Task::start_droppable(async {
        Timer::sleep(1000).await;
        perform_expensive_operation();
    });
    debounce_handle.set(Some(handle));
});

// Compare-before-update optimization
let current = STATE.lock_ref();
if *current != new_value {
    drop(current);
    STATE.set_neq(new_value); // Only trigger if different
}
```

## Testing Reactive Code

### Manual Testing Techniques
```rust
// Add debug counters for signal firing frequency
static DEBUG_COUNTER: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(0));

SIGNAL.signal().for_each_sync(|_| {
    let count = DEBUG_COUNTER.get() + 1;
    DEBUG_COUNTER.set(count);
    zoon::println!("🐛 Signal fired {} times", count);
});

// Trace signal chains
zoon::println!("🔍 Signal chain: {} → processing", std::any::type_name::<SignalType>());
```

### Integration Testing
```rust
// Test initialization order
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_config_initialization_order() {
        // Setup
        initialize_config().await;
        
        // Verify state is populated
        assert!(!TRACKED_FILES.lock_ref().is_empty());
        assert!(SELECTED_SCOPE_ID.get().is_some());
        
        // Test that UI can render without panics
        let _ui = create_main_ui();
    }
}
```

This comprehensive guide should help avoid the reactive pitfalls we've encountered and provide clear patterns for building robust reactive systems in NovyWave.