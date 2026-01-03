# NovyWave Domain Map

**5 business domains + 3 foundational systems across 45 Rust files**

## Core Foundation

### Dataflow (`frontend/src/dataflow/`)
- **Relay<T>** - Event streaming, event-source naming
- **Actor<T>** - State management, NO `.get()`, signal-only
- **ActorVec<T>** - Reactive collections (VecDiff)
- **Atom<T>** - Local UI state only

**Critical:** "Cache Current Values" ONLY inside Actor loops

### App Structure (`frontend/src/app.rs`)
```rust
pub struct NovyWaveApp {
    pub tracked_files: TrackedFiles,
    pub selected_variables: SelectedVariables,
    pub waveform_timeline: WaveformTimeline,
    pub config: AppConfig,
    pub dragging_system: DraggingSystem,
}
```

## Business Domains

### 1. File Management
**Files:** `tracked_files.rs`, `file_management.rs`, `file_picker.rs`

- ActorVec<TrackedFile> + event relays
- Smart labeling, TreeView with `items_signal_vec`
- **APIs:** `files_dropped_relay.send()`, `files.signal_vec()`

### 2. Variable Selection
**Files:** `selected_variables.rs`, `variable_selection_ui.rs`, `format_selection.rs`

- 7+ Actors (variables, scope, search, formats)
- Signal-level filtering (5,371+ variables instant)
- **APIs:** `variable_clicked_relay.send()`, `variables_vec_actor.signal()`

### 3. Timeline Visualization
**Files:** `visualizer/timeline/` (8 files)

- `timeline_actors.rs` - 30+ relays
- `zoom_controller.rs`, `canvas_state.rs`, `panning_controller.rs`
- TimeNs/DurationNs precision, Cache Current Values pattern

### 4. Canvas Rendering
**Files:** `visualizer/canvas/` (5 files) - Fast2D graphics

### 5. Platform Abstraction
**Files:** `platform/` (3 files) - Web vs Desktop

## Foundation Systems

- **Configuration:** ConfigSaver auto-persistence, dock-specific dimensions
- **Dragging:** DraggingSystem with Cache Current Values
- **Connection:** ConnectionAdapter for domain message routing

## Integration Points

| From | To | Mechanism |
|------|-----|-----------|
| Files | Variables | File loading → variable extraction |
| Variables | Timeline | Format changes → `format_updated_relay` |
| Timeline | Canvas | TimelineContext bridges state |
| All | Config | Domain changes → ConfigSaver |
| All | Dragging | Panel dimensions via DraggingSystem |

## Rules (MANDATORY)

✅ NO raw Mutables ✅ Event-source naming ✅ Domain-driven design ✅ Public fields

❌ Manager/Service/Controller ❌ SignalVec→Signal ❌ zoon::Task for events ❌ Data bundling
