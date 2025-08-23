# NovyWave Reactive Architecture

## Current State Analysis

### The Problem: Mixed Imperative/Reactive Patterns

Our current config system uses a hybrid approach that causes issues:

1. **Config Loading**: Imperative `populate_globals_from_config()` function sets initial state
2. **Config Saving**: Reactive signal handlers trigger saves
3. **State Synchronization**: Manual coordination between config loading and UI readiness via `CONFIG_LOADED` gate

This leads to:
- Files stuck at "Starting..." status (backend messages never sent during config restore)
- Complex coordination logic with race condition potential
- Maintenance burden of keeping imperative and reactive paths in sync

### Current File Loading Issue

**Broken Flow (Config Restore):**
```
Config Load → populate_globals_from_config() → init_tracked_files_from_config() 
→ add_tracked_file(LoadingStatus::Starting) → NO BACKEND MESSAGE → STUCK
```

**Working Flow (Manual Loading):**
```  
User Action → add_tracked_file(LoadingStatus::Starting) → UpMsg::LoadWaveformFile 
→ Backend Processing → DownMsg::ParsingStarted → Status Updates → Success
```

## Pure Reactive Architecture Design

### Inspiration from Production Systems

Analysis of mature reactive systems reveals patterns for pure reactive architectures:

1. **No Initialization Gates**: Store creation immediately loads and sets reactive state
2. **Centralized Triggers**: Single module establishes all reactive chains at startup
3. **State-First Design**: Configuration is just another reactive state store
4. **Automatic Persistence**: Changes trigger saves through signal chains, not manual calls

### Core Principle: Config as Reactive Store

Transform config from "something that gets applied" to "reactive state that drives everything":

```rust
// Current (imperative):
load_config() → apply_config() → populate_globals_from_config() → manual setup

// Target (reactive):
CONFIG_STORE initialization → immediate signal setup → automatic state sync
```

## Proposed Architecture

Based on comprehensive analysis, the refactor will transform our dual-system hybrid to pure reactive:

### Current Dual System Issues
- **populate_globals_from_config()** - Single imperative function setting 15+ global variables
- **Two gate systems** - CONFIG_LOADED + CONFIG_INITIALIZATION_COMPLETE with 30+ guard checks
- **Bridge functions** - Manual save_scope_selection(), save_panel_layout(), etc.
- **Derived signals** - Complex system converting global state back to config format

### 1. Pure Reactive Trigger Architecture

Replace the entire imperative population system with centralized reactive triggers:

**Before (Imperative)**:
```rust
load_config() → apply_config() → populate_globals_from_config() → manual population
```

**After (Reactive)**:
```rust
CONFIG_STORE initialization → setup_reactive_triggers() → automatic signal chains
```

### 2. Centralized Triggers Module

Create `frontend/src/config/triggers.rs` with complete reactive coverage:

```rust
pub fn setup_reactive_config_system() {
    setup_file_management_triggers();
    setup_ui_state_triggers();
    setup_panel_layout_triggers();
    setup_timeline_triggers();
    setup_selection_triggers();
    setup_persistence_triggers();
}

fn setup_file_management_triggers() {
    // TRACKED_FILES ← config.session.opened_files + auto-backend-loading
    let opened_files_signal = config_store()
        .session.signal_ref(|s| s.opened_files.signal_vec_cloned())
        .flatten_vec();
        
    Task::start(opened_files_signal.for_each(|file_paths| async move {
        // Convert to TrackedFiles with Starting status
        let tracked_files: Vec<TrackedFile> = file_paths.iter()
            .map(|path| TrackedFile::from_path(path.clone()))
            .collect();
            
        TRACKED_FILES.lock_mut().replace_cloned(tracked_files);
        
        // CRITICAL: Auto-send backend loading messages
        for file_path in file_paths {
            use crate::platform::{Platform, CurrentPlatform};
            let _ = CurrentPlatform::send_message(UpMsg::LoadWaveformFile(file_path)).await;
        }
    }).await);
    
    // SELECTED_VARIABLES ← config.workspace.selected_variables
    let variables_signal = config_store()
        .workspace.signal_ref(|w| w.selected_variables.signal_vec_cloned())
        .flatten_vec();
        
    Task::start(variables_signal.for_each(|variables| async move {
        SELECTED_VARIABLES.lock_mut().replace_cloned(variables);
        
        // Update index for fast lookup
        let index: IndexSet<String> = variables.iter()
            .map(|var| var.unique_id.clone())
            .collect();
        SELECTED_VARIABLES_INDEX.set_neq(index);
    }).await);
}

fn setup_ui_state_triggers() {
    // EXPANDED_SCOPES ← config.workspace.expanded_scopes (with scope_ prefix)
    let expanded_signal = config_store()
        .workspace.signal_ref(|w| w.expanded_scopes.signal_vec_cloned())
        .flatten_vec();
        
    Task::start(expanded_signal.for_each(|scopes| async move {
        let mut expanded_set = IndexSet::new();
        for scope_id in scopes {
            if scope_id.contains('|') {
                expanded_set.insert(format!("scope_{}", scope_id));
            } else {
                expanded_set.insert(scope_id);
            }
        }
        EXPANDED_SCOPES.set_neq(expanded_set);
    }).await);
    
    // FILE_PICKER_EXPANDED ← config.workspace.load_files_expanded_directories
    let picker_expanded_signal = config_store()
        .workspace.signal_ref(|w| w.load_files_expanded_directories.signal_vec_cloned())
        .flatten_vec();
        
    Task::start(picker_expanded_signal.for_each(|directories| async move {
        let expanded_set: IndexSet<String> = directories.into_iter().collect();
        FILE_PICKER_EXPANDED.set_neq(expanded_set);
    }).await);

    // VARIABLES_SEARCH_FILTER ← config.session.variables_search_filter
    let search_signal = config_store()
        .session.signal_ref(|s| s.variables_search_filter.signal_cloned())
        .flatten();
        
    Task::start(search_signal.for_each(|filter| async move {
        VARIABLES_SEARCH_FILTER.set_neq(filter);
    }).await);
}

fn setup_panel_layout_triggers() {
    // Dock mode reactive chain: DockMode → boolean + dimension selection
    let dock_mode_signal = config_store()
        .workspace.signal_ref(|w| w.dock_mode.signal_cloned())
        .flatten();
        
    Task::start(dock_mode_signal.for_each(|dock_mode| async move {
        // Set dock mode boolean
        IS_DOCKED_TO_BOTTOM.set_neq(matches!(dock_mode, DockMode::Bottom));
        
        // Load appropriate dimensions for current dock mode
        let workspace = config_store().workspace.lock_ref();
        let layouts = workspace.panel_layouts.lock_ref();
        
        let (files_width, files_height, name_col_width, value_col_width) = match dock_mode {
            DockMode::Bottom => {
                let dims = layouts.docked_to_bottom.lock_ref();
                (dims.files_panel_width.get(), 
                 dims.files_panel_height.get(),
                 dims.variables_name_column_width.get(), 
                 dims.variables_value_column_width.get())
            }
            DockMode::Right => {
                let dims = layouts.docked_to_right.lock_ref();
                (dims.files_panel_width.get(), 
                 dims.files_panel_height.get(),
                 dims.variables_name_column_width.get(), 
                 dims.variables_value_column_width.get())
            }
        };
        
        FILES_PANEL_WIDTH.set_neq(files_width as u32);
        FILES_PANEL_HEIGHT.set_neq(files_height as u32);
        VARIABLES_NAME_COLUMN_WIDTH.set_neq(name_col_width as u32);
        VARIABLES_VALUE_COLUMN_WIDTH.set_neq(value_col_width as u32);
    }).await);
}

fn setup_timeline_triggers() {
    // Timeline cursor position with NaN validation
    let cursor_signal = config_store()
        .workspace.signal_ref(|w| w.timeline_cursor_position.signal())
        .flatten();
        
    Task::start(cursor_signal.for_each(|position| async move {
        if position.is_finite() {
            TIMELINE_CURSOR_POSITION.set_neq(position);
        }
    }).await);
    
    // Timeline zoom level with NaN validation
    let zoom_signal = config_store()
        .workspace.signal_ref(|w| w.timeline_zoom_level.signal())
        .flatten();
        
    Task::start(zoom_signal.for_each(|zoom| async move {
        if zoom.is_finite() {
            TIMELINE_ZOOM_LEVEL.set_neq(zoom);
        }
    }).await);
    
    // Timeline visible range with validation
    let range_start_signal = config_store()
        .workspace.signal_ref(|w| w.timeline_visible_range_start.signal())
        .flatten();
    let range_end_signal = config_store()
        .workspace.signal_ref(|w| w.timeline_visible_range_end.signal())
        .flatten();
        
    Task::start(range_start_signal.for_each(|start| async move {
        if start.is_finite() {
            TIMELINE_VISIBLE_RANGE_START.set_neq(start);
        }
    }).await);
    
    Task::start(range_end_signal.for_each(|end| async move {
        if end.is_finite() {
            TIMELINE_VISIBLE_RANGE_END.set_neq(end);
        }
    }).await);
}
```

### 3. Complete Reactive Replacement

**Full scope of reactive triggers to replace populate_globals_from_config():**

#### File Management (Most Critical - Fixes Loading Issue)
- `TRACKED_FILES` ← `config.session.opened_files` + **auto-backend-messages**
- `SELECTED_VARIABLES` ← `config.workspace.selected_variables` + index update
- `SELECTED_VARIABLES_INDEX` ← derived from SELECTED_VARIABLES

#### Selection & Expansion State
- `EXPANDED_SCOPES` ← `config.workspace.expanded_scopes` (with scope_ prefix transform)
- `SELECTED_SCOPE_ID` ← `config.workspace.selected_scope_id`
- `FILE_PICKER_EXPANDED` ← `config.workspace.load_files_expanded_directories`

#### Panel Layout (Dock Mode Dependent)
- `IS_DOCKED_TO_BOTTOM` ← `config.workspace.dock_mode` (enum → boolean)
- `FILES_PANEL_WIDTH` ← `config.workspace.panel_layouts[dock_mode].files_panel_width`
- `FILES_PANEL_HEIGHT` ← `config.workspace.panel_layouts[dock_mode].files_panel_height`
- `VARIABLES_NAME_COLUMN_WIDTH` ← `config.workspace.panel_layouts[dock_mode].name_column_width`
- `VARIABLES_VALUE_COLUMN_WIDTH` ← `config.workspace.panel_layouts[dock_mode].value_column_width`

#### Timeline State (With NaN Validation)
- `TIMELINE_CURSOR_POSITION` ← `config.workspace.timeline_cursor_position` (validate finite)
- `TIMELINE_ZOOM_LEVEL` ← `config.workspace.timeline_zoom_level` (validate finite)
- `TIMELINE_VISIBLE_RANGE_START` ← `config.workspace.timeline_visible_range_start` (validate finite)
- `TIMELINE_VISIBLE_RANGE_END` ← `config.workspace.timeline_visible_range_end` (validate finite)

#### Session UI State
- `VARIABLES_SEARCH_FILTER` ← `config.session.variables_search_filter`
- `CURRENT_DIRECTORY` ← `config.session.file_picker.current_directory` (validate exists)
- `LOAD_FILES_SCROLL_POSITION` ← `config.session.file_picker.scroll_position`
- `LOAD_FILES_VIEWPORT_Y` ← `config.session.file_picker.viewport_y`

### 4. Gate System Simplification

**Before**: Dual gates with scattered checks
```rust
if CONFIG_LOADED.get() { /* prevent startup overwrites */ }
if CONFIG_INITIALIZATION_COMPLETE.get() { /* allow persistence */ }
```

**After**: Single coordination point
```rust
// Option A: No gates - immediate reactivity
config_store() initialization → triggers activate immediately

// Option B: Single gate if startup coordination needed
REACTIVE_SYSTEM_READY.set_neq(true) → all triggers activate
```

### 5. Persistence System Redesign

**Current**: Manual bridge functions + derived signals
```rust
save_scope_selection() → CONFIG_STORE update → save_config_to_backend()
save_panel_layout() → CONFIG_STORE update → save_config_to_backend()
```

**Proposed**: Direct reactive persistence
```rust
// Global state changes directly update config
EXPANDED_SCOPES.signal().for_each_sync(|scopes| {
    // Transform and update config directly
    let config_scopes: Vec<String> = scopes.into_iter()
        .map(|s| s.strip_prefix("scope_").unwrap_or(&s).to_string())
        .collect();
    config_store().workspace.lock_mut().expanded_scopes.lock_mut().replace_cloned(config_scopes);
    
    // Auto-save (with debouncing)
    save_config_to_backend();
});
```

## Performance Optimizations

### Potential Removal of Task::next_macro_tick

Current actor model uses `Task::next_macro_tick().await` between message processing to prevent recursive locks. With pure reactive architecture, we might eliminate this:

**Benefits:**
- Faster message processing (no artificial delays)
- Simpler code (no manual yielding)
- More responsive UI updates

**Investigation Plan:**
1. Test removing `next_macro_tick` from reactive chains
2. Monitor for recursive lock panics
3. Use async signal handlers (`for_each` vs `for_each_sync`) to naturally break synchronous chains
4. Measure performance improvements

**Hypothesis:** Pure reactive chains with proper async boundaries might eliminate recursive lock conditions naturally.

## Migration Strategy

### Phase 1: Create Reactive Triggers (Current Implementation)
1. Create `config/triggers.rs` module
2. Set up basic reactive chains for critical functionality
3. Test file loading works through reactive patterns

### Phase 2: Remove Imperative Code
1. Delete `populate_globals_from_config()` function
2. Remove `CONFIG_LOADED` gate from all code paths
3. Eliminate `apply_config()` function entirely
4. Update initialization to call `setup_config_reactivity()` once

### Phase 3: Performance Optimization
1. Remove `Task::next_macro_tick()` from reactive chains
2. Test for recursive lock conditions
3. Measure and verify performance improvements
4. Add async boundaries where needed for lock safety

### Phase 4: Validation
1. Verify all config loading/saving functionality works
2. Test file loading from config restoration
3. Ensure no regressions in UI responsiveness
4. Performance benchmark against current implementation

## Expected Benefits

### 1. Fixes Current Issues
- Files load properly from config (no more "Starting..." stuck state)
- Eliminates coordination complexity between config and UI

### 2. Architecture Improvements  
- Pure reactive data flow (no mixed patterns)
- Centralized reactive logic (easier to understand/maintain)
- Automatic state synchronization (no manual populate/sync functions)

### 3. Performance Gains
- Potential removal of artificial delays (`next_macro_tick`)
- More efficient signal chains (only update what changes)  
- Reduced coordination overhead

### 4. Maintainability
- Single source of truth for reactive patterns
- Easier to add new config fields (just add signal chain)
- Better testability (individual reactive chains)

## Key Success Metrics

1. **Functionality**: Files load from config without "Starting..." issues
2. **Performance**: No recursive lock panics, measurable speed improvements  
3. **Code Quality**: Removal of all imperative populate/sync functions
4. **Maintainability**: New config fields require only signal chain additions

This architecture transforms NovyWave from mixed imperative/reactive to pure reactive, following proven patterns from production systems while addressing our specific performance and maintainability needs.

## Reactive Loop Problem & Solutions

### The File Loading Loop Issue

After implementing reactive triggers, we discovered a critical loop:
**Files go through: Starting → Parsing → Loaded → Starting (infinite loop)**

### Root Cause Analysis

The loop occurs due to **bidirectional reactive synchronization** between Config and State:

```
1. Config → State (via triggers.rs):
   Config.opened_files changes → Trigger fires → TRACKED_FILES.replace_cloned(Starting status)

2. State → Config (via persistence):  
   TRACKED_FILES changes → OPENED_FILES_FOR_CONFIG updates → Config.opened_files updates

3. Loop Creation:
   Config loads ["simple.vcd"] → 
   Triggers set TRACKED_FILES to [simple.vcd: Starting] →
   Backend loads file → File becomes [simple.vcd: Loaded] →
   OPENED_FILES_FOR_CONFIG detects change → Updates Config.opened_files →
   Trigger fires AGAIN → Resets to [simple.vcd: Starting] ← INFINITE LOOP!
```

### Critical Code Locations Causing Loop

1. **TRACKED_FILES.lock_mut().replace_cloned()** - Always fires signals (no deduplication)
2. **OPENED_FILES_FOR_CONFIG derived signal** - Detects file state changes
3. **Config persistence handlers** - Update config when state changes
4. **Reactive triggers** - Reset files to Starting when config changes

### Solution Patterns for Reactive Loops

## 1. **Unidirectional Data Flow (Recommended)**
```
Config → Global State → UI (one direction only)
```

**Implementation:**
- Config → State: Only on initial load (one-shot)
- State → Config: Continuous for persistence  
- Never Config → State after initialization

**Benefits:**
- Eliminates circular dependencies
- State owns runtime data, Config is just storage
- Similar to React/Redux patterns

## 2. **Gate Flag Pattern**
```rust
static UPDATING_FROM_CONFIG: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// In config → state trigger
if !UPDATING_FROM_CONFIG.get() {
    UPDATING_FROM_CONFIG.set(true);
    update_state_from_config();
    UPDATING_FROM_CONFIG.set(false);
}

// In state → config trigger  
if !UPDATING_FROM_CONFIG.get() {
    save_config();
}
```

## 3. **One-Shot with futures_signals Methods**

### Available Methods for One-Shot Operations:
- **`signal.first()`** - Takes only first value, then completes
- **`signal.wait_for(value)`** - Waits until signal equals value, then completes
- **`always(value)`** - Creates constant signal that never changes

### One-Shot Config Loading Pattern:
```rust
// Replace continuous triggers:
CONFIG_LOADED.signal().for_each_sync(|loaded| {
    if loaded { setup_reactive_system(); }
});

// With one-shot patterns:
CONFIG_LOADED.signal().wait_for(true).await;
setup_reactive_system(); // Runs once only

// Or even better, use config directly:
config_store()
    .workspace.signal_ref(|w| !w.opened_files.lock_ref().is_empty())
    .first().await;  // Wait for actual config data
setup_one_time_initialization();
```

## 4. **State Preservation Pattern**
```rust
// Instead of always resetting to Starting:
let tracked_files = file_paths.map(|path| {
    create_tracked_file(path, FileState::Starting)  // Always Starting
});

// Preserve existing file states:
let tracked_files = file_paths.map(|path| {
    // Check if file already exists and preserve its state
    if let Some(existing) = find_existing_file(path) {
        existing.clone()  // Keep current state (Parsing/Loaded)
    } else {
        create_tracked_file(path, FileState::Starting)  // Only new files
    }
});
```

## 5. **Compare Before Update Pattern**
```rust
// Instead of always replacing (triggers signals):
TRACKED_FILES.lock_mut().replace_cloned(new_files);

// Check if actually different first:
let current = TRACKED_FILES.lock_ref();
if *current != new_files {
    drop(current);
    TRACKED_FILES.lock_mut().replace_cloned(new_files);
}
```

### Recommended Solution for NovyWave

**Use Unidirectional Flow with One-Shot Config Loading:**

1. **Remove reactive Config → State triggers** after initial load
2. **Use `.first()` or `.wait_for()`** for one-time initialization
3. **Preserve file states** instead of resetting to Starting
4. **Keep State → Config persistence** for user changes

```rust
// One-time config initialization
Task::start(async {
    // Wait for config to have actual data
    config_store()
        .workspace.signal_ref(|w| !w.opened_files.lock_ref().is_empty())
        .first().await;
    
    // Initialize state from config ONCE
    initialize_state_from_config_preserving_states();
    
    // Setup only state → config persistence (not config → state)
    setup_state_to_config_persistence();
});
```

**Result:** Files load once and stay loaded without looping back to Starting status.

### Performance Notes

- **Task::next_macro_tick().await** might be removable with pure reactive architecture
- One-shot patterns are more performant than continuous reactive chains
- Unidirectional flow eliminates unnecessary state synchronization overhead

This eliminates reactive loops while maintaining proper config persistence and improving performance.