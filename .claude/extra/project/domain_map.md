# NovyWave Domain Map & Architecture Guide

**PURPOSE:** Comprehensive domain mapping to prevent duplicated analysis and enable efficient task execution.

## Domain Architecture Overview

NovyWave uses **Actor+Relay architecture** with 5 major business domains + 3 foundational systems across 45 Rust files. This map provides instant context for any development task.

## Core Foundation Layer (UNDERSTAND FIRST)

### Dataflow Architecture (`frontend/src/dataflow/`)
**Files:** `mod.rs`, `actor.rs`, `actor_vec.rs`, `atom.rs`, `relay.rs`, `actor_map.rs` (6 files)

**Key Types & Responsibilities:**
- **`Relay<T>`** - Event streaming with single-source constraint (event-source naming required)
- **`Actor<T>`** - Sequential state management, NO `.get()` methods, signal-only access  
- **`ActorVec<T>`** - Reactive collections with VecDiff efficiency
- **`ActorMap<K,V>`** - Reactive maps with MapDiff updates
- **`Atom<T>`** - Local UI state wrapper (NOT for domain data)

**Critical Pattern:** "Cache Current Values" ONLY allowed inside Actor processing loops

### Application Structure (`frontend/src/app.rs`)
**Main Structure:**
```rust
pub struct NovyWaveApp {
    // Business domains (self-contained)
    pub tracked_files: TrackedFiles,
    pub selected_variables: SelectedVariables, 
    pub waveform_timeline: WaveformTimeline,
    pub waveform_canvas: WaveformCanvas,
    
    // Foundation systems
    pub config: AppConfig,
    pub dragging_system: DraggingSystem,
    pub connection: Arc<Connection<UpMsg, DownMsg>>,
}
```
**Pattern:** Domain parameter passing eliminates 74+ raw global Mutables

### Configuration System (`frontend/src/config.rs`)
**Architecture:** Dual-layer (shared types + reactive frontend)
- **Config Actors:** Individual actors per concern (theme, dock_mode, 10+ dimensions)
- **Event-source relays:** `theme_button_clicked_relay`, `dock_mode_button_clicked_relay`
- **ConfigSaver Actor:** Automatic debounced persistence (1-second delay)
- **TreeView Integration:** Sync actors bridge Mutable<T> requirements to config system

## Business Domains

### 1. File Management Domain
**Primary:** `tracked_files.rs` (domain Actor+Relay)
**Supporting:** `file_management.rs` (UI), `file_operations.rs` (utilities), `file_picker.rs` (selection dialog)

**Key Architecture:**
- **TrackedFiles struct** with ActorVec<TrackedFile> + multiple event relays
- **Smart labeling** with on-demand computation (not complex signal coordination)
- **TreeView integration** using `items_signal_vec` pattern (NOT `child_signal` antipattern)
- **Internal relay pattern** for async operations (eliminates zoon::Task usage)

**Key APIs:**
```rust
// Event emission
tracked_files.files_dropped_relay.send(vec![path]);
tracked_files.file_load_completed_relay.send((file_id, state));

// Signal access  
tracked_files.files.signal_vec() // VecDiff efficiency
tracked_files.files_vec_signal.signal_cloned() // Stable Vec signal
```

**Data Flow:** User Interaction → file_picker.rs → file_operations.rs → tracked_files.rs → Backend

### 2. Variable Selection Domain
**Primary:** `selected_variables.rs` (comprehensive domain Actor+Relay)
**Supporting:** `variable_selection_ui.rs` (multi-layout components), `format_selection.rs` (dropdown system)

**Key Architecture:**
- **SelectedVariables struct** with 7+ Actors (variables, scope, search, tree state, formats)
- **Signal-level performance:** Separates expensive data loading from cheap filtering
- **VecDiff → Vec sync:** Dedicated actor solves SignalVec conversion antipattern
- **Multi-layout UI:** Basic browsing + complex waveform integration + dock-responsive

**Performance Achievement:** Instant filtering of 5,371+ variables through signal-level architecture

**Key APIs:**
```rust
// Domain events
selected_variables.variable_clicked_relay.send(var_id);
selected_variables.variable_format_changed_relay.send((id, format));

// Signal access
selected_variables.variables.signal_vec() // Individual variable updates
selected_variables.variables_vec_actor.signal() // Stable Vec<SelectedVariable>
```

### 3. Timeline Visualization Domain
**Directory:** `visualizer/timeline/` (8 files - modularized from 1,593 → 908 lines, 57% reduction)

**Modular Controllers:**
- **`timeline_actors.rs`** - Central coordination (30+ relays for user interactions)
- **`zoom_controller.rs`** - Complete zoom management (282 lines)
- **`canvas_state.rs`** - Rendering coordination (195 lines)  
- **`panning_controller.rs`** - Left/right viewport movement (95 lines)
- **`cursor_animation.rs`** - Smooth cursor movement (140 lines)
- **`timeline_cache.rs`** - Unified caching system (169 lines)
- **`maximum_timeline_range.rs`** - Standalone derived state actor

**Key Architecture:**
- **Time precision:** Nanosecond-level TimeNs/DurationNs types
- **Coordinate systems:** Time ↔ pixel conversions via NsPerPixel
- **Cache Current Values:** Exemplary implementation in multiple controllers
- **30+ Event Relays:** Comprehensive user interaction coverage

**Integration:** TimelineContext provides canvas with domain access

### 4. Canvas Rendering Domain
**Directory:** `visualizer/canvas/` (5 files)
**Integration:** Fast2D graphics + timeline state coordination + user interactions

### 5. Platform Abstraction Domain
**Directory:** `platform/` (3 files)
**Architecture:** Compile-time selection between Web (MoonZoon Connection) and Desktop (Tauri IPC)

## Foundation Systems

### Configuration Management
- **Dock-specific dimensions:** Prevents sync issues between Right/Bottom modes
- **Actor state collection:** Uses `.signal().to_stream().next().await` for config assembly
- **Bi-directional sync:** TreeView Mutables ↔ SessionState ↔ Config persistence

### Dragging System (`dragging.rs`)
**Architecture:** Pure Actor+Relay with exemplary "Cache Current Values" implementation
- **Centralized state:** DraggingSystem with drag_state_actor
- **Config integration:** Real-time dimension updates + debounced persistence
- **Multi-divider support:** Files panel, variables columns, timeline panels

### Connection System (`connection.rs`)  
**Pattern:** ConnectionAdapter creates Actor-compatible message streams
- **Direct domain routing:** DownMsg variants invoke specific domain relays
- **Platform abstraction:** Same UpMsg/DownMsg works on web + desktop
- **Type safety:** Compiler ensures message→domain routing

## UI & Layout Systems

### Panel Layout (`panel_layout.rs`)
- **Standardized panel pattern** with theme-aware styling
- **Integrated divider system** connecting to DraggingSystem
- **Responsive design** with Height::fill() inheritance chains

### Virtual Lists & Performance (`virtual_list.rs`)
- **Stable element pools** - update content only, never recreate DOM
- **Velocity-based buffering** (5-15 elements, not 50+ causing slowdowns)

## Critical Architectural Rules (ALWAYS FOLLOW)

### MANDATORY Patterns
✅ **NO RAW MUTABLES** - Use Actor+Relay or Atom only  
✅ **Event-source relay naming** - `file_dropped_relay` NOT `add_file_relay`  
✅ **Domain-driven design** - TrackedFiles NOT FileManager  
✅ **Cache Current Values** - ONLY inside Actor loops  
✅ **Public field architecture** - Direct access, no unnecessary getters

### PROHIBITED Antipatterns  
❌ **Manager/Service/Controller abstractions** - Objects manage data, not other objects  
❌ **SignalVec→Signal conversion** - Use items_signal_vec or dedicated sync actors  
❌ **zoon::Task for event handling** - Use Actors with internal relays instead  
❌ **Global Mutables** - 74+ were migrated to proper Actor+Relay  
❌ **Data bundling structs** - Don't force unrelated data to update together

## Task Execution Strategy

### For ANY Development Task:

1. **Domain Identification** - Which of the 5 domains does this affect?
2. **Architectural Pattern** - What Actor+Relay pattern applies?
3. **Existing Implementation** - Reference this map instead of exploring
4. **Cross-domain Impact** - Check integration points listed above
5. **Verification** - Browser MCP testing + compilation check

### Subagent Delegation Rules:
- **Use for:** Multi-domain analysis, performance debugging, architectural compliance
- **Avoid for:** Single-file edits, known pattern application, simple config changes

### Context Conservation:
- **Reference this map** instead of re-analyzing domains
- **Use established patterns** documented above
- **Work domain-cohesively** rather than scattered file changes

## Integration Points Reference

### File Management ↔ Variable Selection
- File loading state affects variable extraction
- File removal triggers variable cleanup
- Smart labeling considers file contexts

### Variable Selection ↔ Timeline  
- Format changes sent via `variable_format_updated_relay`
- Selected variables affect timeline data requests
- Cursor position updates variable value display

### Timeline ↔ Canvas
- TimelineContext bridges domain state to rendering
- Coordinate translation via TimelineCoordinates system
- Performance tracking and cache coordination

### All Domains ↔ Configuration
- Domain state changes trigger ConfigSaver debounced persistence  
- Config restoration on startup populates domain actors
- Dock mode affects UI layouts across multiple domains

### All Domains ↔ Dragging
- Panel dimensions managed through DraggingSystem
- Real-time updates via config actors
- Cache Current Values pattern for smooth interactions

This map eliminates context switching and prevents architectural violations by providing instant domain understanding and established patterns for any development task.