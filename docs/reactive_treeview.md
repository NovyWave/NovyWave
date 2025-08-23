# ReactiveTreeView: Ultra-Granular Signal-Friendly TreeView API

## Motivation & Problem Analysis

### Current Over-Rendering Crisis

**Symptoms Observed:**
- **30+ TreeView renders** within 300ms during file loading operations
- **UI flickering and browser lag** during TreeView interactions
- **Signal cascade pattern**: `TRACKED_FILES → SMART_LABELS → child_signal(map_ref!) → Full TreeView Recreation`
- **Bulk update inefficiency**: Every file state change (Loading→Loaded) rebuilds entire tree
- **Performance degradation**: Poor user experience with multiple files loaded

**Root Cause Analysis:**
1. **Bulk Update Pattern**: Current TreeView uses `.data(Vec<TreeViewItemData>)` causing complete DOM recreation
2. **Signal Cascade**: Intermediate signals trigger multiple full tree rebuilds  
3. **State Change Over-Reaction**: Minor file state changes trigger complete tree reconstruction
4. **No Granular Updates**: Cannot update individual tree items - always full replacement

### Current TreeView API Limitations

**NovyUI TreeView Current API:**
```rust
tree_view()
    .data(Vec<TreeViewItemData>)           // ❌ Static data only, no signal support
    .external_expanded(signal)             // ✅ External state works  
    .external_selected(signal)             // ✅ External state works
```

**Critical Problems:**
- **No `.data_signal()` support** - cannot reactively update tree content
- **Bulk updates only** - complete DOM recreation on every data change
- **No differential rendering** - cannot add/remove/modify individual items
- **No stable element identity** - loses internal state across updates
- **Mixed data challenges** - Files + Scopes in same tree problematic

### Performance Impact Evidence

**Current Inefficiency Pattern:**
```rust
// Every file state change triggers this entire chain
TRACKED_FILES.signal_vec_cloned().to_signal_cloned().map(|tracked_files| {
    // ❌ Recomputes ALL smart labels on every change
    let smart_labels = shared::generate_smart_labels(&paths);
    // ❌ Converts ALL files to tree data structure  
    let tree_data = convert_tracked_files_to_tree_data(&tracked_files, &smart_labels);
    // ❌ Creates BRAND NEW TreeView instance from scratch
    tree_view().data(tree_data)
})
```

**Real-World Impact Measured:**
- 1 file state change → All 5+ files get reprocessed and re-rendered
- Tree expansion state sometimes lost during updates
- Browser performance degradation with larger file sets
- User experience becomes sluggish during file loading sequences

## Dual TreeView Context Analysis

### Context 1: Files & Scopes Panel

**Current Location**: `frontend/src/views.rs:1319`

**Data Architecture:**
- **Primary Data**: `TRACKED_FILES: MutableVec<TrackedFile>`
- **Secondary Data**: Smart labels computed on-demand from file paths
- **Hierarchy Pattern**: File → Scope → Nested Scopes (via `convert_scope_to_tree_data`)
- **State Management**: External state with dual-layer approach

**Update Characteristics:**
- **High-Frequency Updates**: File loading states, scope discovery, real-time progress
- **Selection Model**: `single_scope_selection(true)` + `show_checkboxes_on_scopes_only(true)`
- **Expansion State**: Persistent via `EXPANDED_SCOPES_FOR_TREEVIEW` (config-backed)
- **Performance Critical**: Needs deduplication and immediate sync patterns

**Current Data Flow:**
```
TRACKED_FILES.signal_vec_cloned().to_signal_cloned()
  → compute smart_labels on-demand (avoid intermediate statics)
  → convert_tracked_files_to_tree_data(&tracked_files, &smart_labels)  
  → TreeView with external_expanded/external_selected
```

### Context 2: Load Files Dialog

**Current Location**: `frontend/src/views.rs:2425`

**Data Architecture:**
- **Primary Data**: `FILE_TREE_CACHE: HashMap<String, Vec<FileSystemItem>>`
- **Secondary Data**: Error cache for directory access failures
- **Hierarchy Pattern**: Directory → Subdirectory → Files (via `build_hierarchical_tree`)
- **State Management**: External state with path-based keys

**Update Characteristics:**
- **Lower-Frequency Updates**: User-driven navigation, directory expansion
- **Selection Model**: Multi-select files via `external_selected_vec(FILE_PICKER_SELECTED)`
- **Expansion State**: Session-only via `FILE_PICKER_EXPANDED` (not persisted)
- **Performance Needs**: Large directory handling, error state management

**Current Data Flow:**
```
FILE_TREE_CACHE.signal_ref(|cache| cache.clone())
  → build_hierarchical_tree("/", &tree_cache, &error_cache)
  → TreeView with FILE_PICKER_EXPANDED/FILE_PICKER_SELECTED
```

### Key Architectural Differences

| Aspect | Files & Scopes Panel | Load Files Dialog |
|--------|---------------------|-------------------|
| **Data Model** | Complex nested objects with state transitions | Simple filesystem representation |
| **Update Frequency** | High (real-time file loading) | Low (user navigation) |
| **Selection** | Single scope with checkboxes | Multi-file selection |
| **Hierarchy** | File → Dynamic scopes | Directory → Static files |
| **Performance Risk** | Signal cascades, over-rendering | Large directory listings |
| **State Persistence** | Config-backed expansion state | Session-only navigation |

## ReactiveTreeView Solution Design

### Core Architecture Principles

1. **Ultra-Granular Updates**: Only changed items affect DOM, never full recreation
2. **Signal-Native Design**: First-class reactive programming support
3. **Dual-Context Support**: Handle both Files & Scopes and Load Files elegantly
4. **Stable Identity Tracking**: Items tracked by keys for efficient diffing
5. **Drop-in Migration**: Easy replacement for existing TreeView usage

### Unified API Design

```rust
ReactiveTreeView::builder()
    // === FLEXIBLE DATA SOURCE ===
    .data_source(match context {
        Context::FilesAndScopes => DataSource::from_signal_vec(TRACKED_FILES.signal_vec_cloned()),
        Context::LoadFiles => DataSource::from_cache(FILE_TREE_CACHE.signal_ref(|c| c.clone())),
    })
    
    // === IDENTITY & DIFFING ===
    .item_key(|item| match context {
        Context::FilesAndScopes => match item {
            TreeItem::File(f) => f.id.clone(),
            TreeItem::Scope(s) => format!("{}|{}", s.file_id, s.scope_path),
        },
        Context::LoadFiles => item.path.clone(),
    })
    
    // === CONTEXT-AWARE RENDERING ===
    .item_renderer(|item| match (item, context) {
        (TreeItem::File(file), Context::FilesAndScopes) => TreeItemBuilder::new()
            .label(&file.smart_label)
            .icon(match file.state {
                FileState::Loading => IconName::Spinner,
                FileState::Loaded => IconName::File,
                FileState::Error => IconName::Warning,
            })
            .tooltip(&file.path),
            
        (FileSystemItem::Directory(dir), Context::LoadFiles) => TreeItemBuilder::new()
            .label(&dir.name)
            .icon(IconName::Folder)
            .expandable(true)
            .on_expand(move |path| load_directory_contents(path)),
    })
    
    // === CONTEXT-SPECIFIC HIERARCHY ===
    .hierarchy_provider(|item| match context {
        Context::FilesAndScopes => extract_scope_parent(item),
        Context::LoadFiles => item.path.parent().map(|p| p.to_string()),
    })
    
    // === EXTERNAL STATE (PRESERVED) ===
    .external_expanded_signal(context.expanded_state())
    .external_selected_signal(context.selected_state())
    
    // === CONTEXT-SPECIFIC BEHAVIOR ===
    .selection_mode(match context {
        Context::FilesAndScopes => SelectionMode::SingleScope,
        Context::LoadFiles => SelectionMode::MultiFile,
    })
    .build()
```

### Update Behavior Transformation

**Current TreeView (Inefficient):**
```rust
// File state change: Loading → Loaded  
TRACKED_FILES changes → Signal fires → ALL files reprocessed → Entire TreeView recreated
```

**ReactiveTreeView (Ultra-Efficient):**
```rust
// File state change: Loading → Loaded
file.state changes → Only that file's renderer called → Only that DOM node updates
```

**Detailed Update Scenarios:**

| Change Type | Current Behavior | ReactiveTreeView Behavior |
|-------------|------------------|--------------------------|
| File state change (Loading→Loaded) | Full tree rebuild (30+ renders) | Single icon update |
| File added to collection | Full tree rebuild | Single item insertion |
| File removed from collection | Full tree rebuild | Single item removal |
| Smart label disambiguation | Full tree rebuild | Affected label updates only |
| Multiple files loading sequence | 30+ rebuilds in 300ms | N individual updates |
| Scope expansion in config | Full tree rebuild on restore | Preserved expansion state |

## Implementation Architecture

### Module Structure

```
novyui/moonzoon-novyui/src/components/reactive_tree_view/
├── mod.rs                 # Public API exports
├── component.rs           # Main ReactiveTreeView struct
├── builder.rs             # Fluent API builder methods  
├── context.rs             # TreeViewContext and SelectionMode enums
├── data_source.rs         # DataSource abstraction for different signal types
├── differ.rs              # Efficient item diffing algorithm
├── dom_updater.rs         # Minimal DOM manipulation system
└── item_renderer.rs       # TreeItemRenderer and TreeItemBuilder
```

### Core Components Deep Dive

#### 1. DataSource Abstraction
```rust
pub enum DataSource<T> {
    /// Reactive collections (e.g., TRACKED_FILES)
    SignalVec(SignalVec<T>),
    
    /// Cached collections (e.g., FILE_TREE_CACHE)  
    Signal(Box<dyn Signal<Item = Vec<T>> + Unpin>),
    
    /// Static data for testing/simple cases
    Static(Vec<T>),
}
```

#### 2. TreeViewContext System
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeViewContext {
    /// Files & Scopes: Complex nested objects, real-time updates
    FilesAndScopes,
    
    /// Load Files: Simple filesystem, user-driven navigation  
    LoadFiles,
}
```

#### 3. Efficient Diffing Algorithm
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TreeChange {
    Add { key: String, item: TreeItem, parent_key: Option<String>, index: Option<usize> },
    Remove { key: String },
    Update { key: String, old_item: TreeItem, new_item: TreeItem },
    Move { key: String, old_parent_key: Option<String>, new_parent_key: Option<String> },
}
```

#### 4. DOM Update Strategy
```rust
impl DomUpdater {
    /// Apply minimal DOM changes - only affected elements
    pub fn apply_changes(&mut self, changes: Vec<TreeChange>) {
        for change in changes {
            match change {
                TreeChange::Add { key, item, parent_key, index } => {
                    // Insert single DOM node at specific position
                },
                TreeChange::Update { key, new_item, .. } => {
                    // Update existing DOM node content only
                },
                TreeChange::Remove { key } => {
                    // Remove single DOM node
                },
                TreeChange::Move { key, new_parent_key, .. } => {
                    // Move DOM node to new parent without recreation
                },
            }
        }
    }
}
```

## Migration Strategy

### Phase 1: Files & Scopes Panel Migration (Medium Complexity)

**Current Pattern (Over-Rendering):**
```rust
// frontend/src/views.rs:1319
.child_signal(
    TRACKED_FILES.signal_vec_cloned().to_signal_cloned().map(|tracked_files| {
        // ❌ Full recreation every time
        tree_view().data(convert_tracked_files_to_tree_data(&tracked_files))
    })
)
```

**New Pattern (Ultra-Granular):**
```rust
.child(
    reactive_tree_view()
        .for_files_and_scopes()
        .data_source(DataSource::from_signal_vec(TRACKED_FILES.signal_vec_cloned()))
        .item_renderer(create_tracked_file_renderer())
        .external_expanded_signal(EXPANDED_SCOPES_FOR_TREEVIEW.clone())
        .external_selected_signal(TREE_SELECTED_ITEMS.clone())
)
```

**Migration Benefits:**
- ✅ Eliminates 30+ renders per file loading sequence
- ✅ Preserves existing external state patterns (`EXPANDED_SCOPES_FOR_TREEVIEW`)
- ✅ Maintains smart label computation (on-demand, no caching needed)
- ✅ Keeps scope expansion/selection behavior identical
- ✅ Solves config restoration timing issues

### Phase 2: Load Files Dialog Migration (Low Complexity)

**Current Pattern (Cache-Based):**
```rust
// frontend/src/views.rs:2425
FILE_TREE_CACHE.signal_ref(|cache| {
    tree_view().data(build_hierarchical_tree("/", cache))
})
```

**New Pattern (Reactive Cache):**
```rust
reactive_tree_view()
    .for_load_files()
    .data_source(DataSource::from_signal(FILE_TREE_CACHE.signal()))
    .item_renderer(create_file_system_renderer())
    .external_expanded_signal(FILE_PICKER_EXPANDED.signal())
    .external_selected_signal(FILE_PICKER_SELECTED.signal_vec_cloned())
```

**Migration Benefits:**
- ✅ Better large directory handling with potential virtual scrolling
- ✅ Preserved error handling patterns for inaccessible directories
- ✅ Same multi-file selection behavior
- ✅ Improved performance for directory navigation
- ✅ Foundation for drag-and-drop file operations

### Phase 3: Config Integration Verification

**Critical Requirements:**
- ✅ Expanded scopes restoration from `.novywave` config must work
- ✅ `EXPANDED_SCOPES_FOR_TREEVIEW` immediate sync pattern preserved
- ✅ Config loading doesn't trigger unnecessary ReactiveTreeView updates
- ✅ Tree expansion state preserved during granular updates
- ✅ No timing issues between config load and tree initialization

## Browser Testing Strategy

### Why Browser Testing is Essential

**Unit tests cannot validate:**
- WASM compilation and execution environment
- Zoon signal behavior in browser context
- DOM manipulation and update performance
- Real user interaction patterns
- Browser-specific rendering optimizations

### Testing Approach

**Phase 1: Basic Functionality Test**
1. Load NovyWave in browser (`makers start`)
2. Add ReactiveTreeView alongside existing TreeView
3. Verify compilation succeeds without errors
4. Test basic rendering with static data

**Phase 2: Signal Integration Test**
1. Connect ReactiveTreeView to `TRACKED_FILES` signal
2. Load multiple waveform files
3. Monitor console for rendering frequency
4. Verify <5 renders per actual data change

**Phase 3: Context Switching Test**
1. Test Files & Scopes panel integration
2. Test Load Files dialog integration
3. Verify both contexts work independently
4. Check external state synchronization

**Phase 4: Performance Validation**
1. Load 5+ files with complex scope hierarchies
2. Monitor rendering performance during file loading
3. Test expansion state preservation during updates
4. Verify config restoration works correctly

**Phase 5: Edge Case Testing**
1. Large file sets (10+ files)
2. Deep scope hierarchies (5+ levels)
3. Rapid file state changes (Loading→Loaded→Error)
4. Config restoration with many expanded scopes

### Success Metrics

**Performance Improvements:**
- **Before**: 30+ TreeView renders during file loading
- **After**: ≤1 render per actual data change
- **Before**: Full tree recreation on file state changes  
- **After**: Single icon/label updates only
- **Before**: UI flickering during operations
- **After**: Smooth, responsive interactions

**Functional Preservation:**
- ✅ Files & Scopes expansion state preserved across updates
- ✅ Load Files multi-selection behavior unchanged
- ✅ Config persistence works identically to current implementation
- ✅ Error handling patterns maintained for both contexts
- ✅ Keyboard navigation and accessibility preserved

## Implementation TODOs

### Phase 1: Core Component Foundation ✅ COMPLETED
- [x] Design `DataSource<T>` abstraction for different data types
- [x] Create `TreeViewContext` enum (FilesAndScopes, LoadFiles)
- [x] Implement efficient item diffing algorithm using stable keys
- [x] Build minimal DOM update system (add/remove/update individual nodes)
- [x] Create flexible `TreeItemRenderer` system for different contexts
- [x] Set up `novyui/src/reactive_tree_view/` module structure
- [x] Create main `ReactiveTreeView` component struct with builder pattern

### Phase 2: Browser Integration & Testing 🔄 NEXT
- [ ] Fix compilation issues with Zoon integration
- [ ] Create test renderer functions for TrackedFile context
- [ ] Add ReactiveTreeView to NovyWave UI for browser testing
- [ ] Verify signal handling works correctly in WASM
- [ ] Test basic rendering and interaction in browser
- [ ] Performance monitoring with browser dev tools

### Phase 3: Files & Scopes Integration
- [ ] Create `TrackedFileRenderer` implementation
- [ ] Build `FilesAndScopesContext` with proper hierarchy logic
- [ ] Migrate `views.rs:1319` to use ReactiveTreeView
- [ ] Preserve `EXPANDED_SCOPES_FOR_TREEVIEW` external state
- [ ] Remove old signal cascade patterns causing over-rendering
- [ ] Verify file loading performance improvement

### Phase 4: Load Files Integration
- [ ] Create `FileSystemRenderer` implementation  
- [ ] Build `LoadFilesContext` with directory hierarchy logic
- [ ] Migrate `views.rs:2425` to use ReactiveTreeView
- [ ] Preserve `FILE_PICKER_EXPANDED` and `FILE_PICKER_SELECTED` state
- [ ] Test large directory handling (1000+ files)
- [ ] Verify error state handling for inaccessible directories

### Phase 5: Config & Persistence
- [ ] Test expanded scopes restoration from `.novywave` config
- [ ] Verify `EXPANDED_SCOPES_FOR_TREEVIEW` immediate sync pattern
- [ ] Fix any timing issues between config load and tree initialization
- [ ] Ensure config loading doesn't trigger unnecessary renders
- [ ] Add debug logging for config → tree state flow

### Phase 6: Advanced Features & Polish
- [ ] Implement optional virtual scrolling for large datasets
- [ ] Add comprehensive performance benchmarking vs old TreeView
- [ ] Optimize memory usage for large file sets
- [ ] Create comprehensive browser compatibility test suite
- [ ] Document best practices for ReactiveTreeView usage

## Technical Considerations

### Memory Management
- **Item Caching**: Cache rendered TreeItem content for unchanged items to avoid recomputation
- **Signal Cleanup**: Properly dispose of item-level signals when items are removed from collection
- **DOM Recycling**: Reuse DOM nodes when possible during updates to minimize allocation

### Error Handling
- **Invalid Hierarchy**: Handle circular parent references gracefully with cycle detection
- **Missing Keys**: Fallback behavior when item_key function returns duplicates
- **Render Failures**: Graceful degradation when item_renderer throws exceptions
- **Signal Errors**: Proper error boundaries for signal processing failures

### Browser Compatibility
- **Signal Performance**: Ensure efficient signaling across different browsers and devices
- **DOM Update Speed**: Test minimal update performance on slower devices and older browsers
- **Memory Leaks**: Verify proper cleanup of signals and DOM references during component destruction
- **WASM Integration**: Test ReactiveTreeView behavior in different WASM environments

## Future Enhancement Roadmap

### Advanced Signal Patterns
```rust
reactive_tree_view()
    // Per-item signals for ultra-granular control
    .item_signal(|file| file.state.signal().map(|state| render_state_icon(state)))
    
    // Conditional rendering based on filters  
    .filter_signal(search_query.signal().map(|query| move |file| file.matches(query)))
    
    // Sorting without full recreation
    .sort_signal(sort_criteria.signal().map(|criteria| sort_comparator(criteria)))
```

### Integration with Other Components
- **VirtualList**: Hybrid ReactiveTreeView + VirtualList for massive datasets (1000+ files)
- **DragAndDrop**: Native drag-and-drop integration for file reordering and organization
- **ContextMenu**: Right-click integration for file operations and scope management

### Performance Optimizations
- **Web Workers**: Move heavy computations (smart labels, hierarchy building) to background threads
- **RequestAnimationFrame**: Batch DOM updates within animation frames for 60fps performance
- **Intersection Observer**: Only render visible tree items in large hierarchies to reduce DOM overhead

---

## Summary

ReactiveTreeView solves the fundamental over-rendering issues plaguing NovyWave's TreeView components by:

1. **Eliminating bulk updates** - only changed items affect DOM, never full recreation
2. **Native signal integration** - no manual conversion or intermediate signals needed  
3. **Stable identity tracking** - preserves state across updates using efficient diffing
4. **Separation of concerns** - clean separation between content, hierarchy, and state management
5. **Drop-in compatibility** - easy migration path from existing TreeView usage

**Expected Impact:**
- **Performance**: 30+ renders → 1 render per actual change
- **User Experience**: Smooth file loading experience without UI flickering
- **Maintainability**: Foundation for future advanced TreeView features
- **Architecture**: Clean, extensible design supporting multiple contexts

This design provides the ultra-granular, signal-friendly TreeView that NovyWave needs for professional waveform viewer performance while maintaining simplicity, reliability, and extensibility for future enhancements.