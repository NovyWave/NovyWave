# NovyWave Domain Map

**5 business domains + 3 foundational systems across 45 Rust files**

## Core Foundation

### Pure Reactive Dataflow
- **Mutable<T>** - State container, state change IS the event
- **Signal** - Reactive observation via `.signal()` / `.signal_cloned()`
- **map_ref!** - Derived state from multiple signals
- **for_each_sync()** - Synchronous side effects on signal changes
- **MutableVec<T>** - Reactive collections with `signal_vec_cloned()`

**Critical:** State changes propagate via signals - NO imperative method calls between domains

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

- `timeline_actor.rs` - WaveformTimeline with TimelineInputs (pure dataflow)
- `TimelineInputs` - Mutables for keyboard/canvas/format events
- External code sets `timeline.inputs.*`, Timeline observes and reacts
- TimePs precision, viewport/cursor state via Mutables

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
| Variables | Timeline | `timeline.inputs.format_update.set(...)` |
| Keyboard | Timeline | `timeline.inputs.cursor_move_request.set(...)` |
| Canvas | Timeline | `timeline.inputs.canvas_dimensions.set(...)` |
| Connection | TrackedFiles | `backend_messages.file_loaded.set(...)` |
| All | Config | ConfigSaver observes domain Mutables via `map_ref!` |

## Rules (MANDATORY)

✅ Pure signal dataflow ✅ Mutable<T> for state ✅ Domain-driven design ✅ Public fields

❌ Imperative method calls between domains ❌ SignalVec→Signal conversion ❌ mpsc channels for state
